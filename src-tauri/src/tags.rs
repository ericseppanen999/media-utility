//! Cover art preparation and verified metadata writing.

use crate::domain::*;
use image::ImageDecoder;
use lofty::{
    config::WriteOptions,
    file::{AudioFile, TaggedFileExt},
    picture::{Picture, PictureType},
    tag::{ItemKey, Tag, TagType},
};
use std::{io::Cursor, path::Path};

/// Output extensions with a verified metadata adapter.
pub const TAGGABLE_EXTENSIONS: [&str; 5] = ["mp3", "m4a", "flac", "ogg", "wav"];

const MAX_ARTWORK_BYTES: u64 = 20 * 1024 * 1024;
const MAX_ARTWORK_PIXELS: u64 = 40_000_000;
const MAX_ARTWORK_EDGE: u32 = 1500;

fn key(name: &str) -> Option<ItemKey> {
    Some(match name {
        "title" => ItemKey::TrackTitle,
        "artist" => ItemKey::TrackArtist,
        "album_artist" => ItemKey::AlbumArtist,
        "album" => ItemKey::AlbumTitle,
        "track" => ItemKey::TrackNumber,
        "track_total" => ItemKey::TrackTotal,
        "disc" => ItemKey::DiscNumber,
        "disc_total" => ItemKey::DiscTotal,
        "year" => ItemKey::Year,
        "genre" => ItemKey::Genre,
        "comment" => ItemKey::Comment,
        _ => return None,
    })
}

fn artwork_error(error: impl ToString) -> AppError {
    AppError::new("ARTWORK", error)
}

/// Validates an image and stores a normalized PNG copy, returning the stored path.
pub fn prepare_artwork(path: &Path, managed: &Path) -> Result<String> {
    if path.metadata()?.len() > MAX_ARTWORK_BYTES {
        return Err(artwork_error("Artwork must be smaller than 20 MB"));
    }
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Jpeg | image::ImageFormat::Png)
    ) {
        return Err(artwork_error("Choose a JPEG or PNG image"));
    }
    let (width, height) = image::ImageReader::open(path)?
        .with_guessed_format()?
        .into_dimensions()
        .map_err(artwork_error)?;
    if (width as u64) * (height as u64) > MAX_ARTWORK_PIXELS {
        return Err(artwork_error("Artwork must be smaller than 40 megapixels"));
    }

    let mut decoder = reader.into_decoder().map_err(artwork_error)?;
    let orientation = decoder.orientation().map_err(artwork_error)?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(artwork_error)?;
    image.apply_orientation(orientation);

    let original = std::fs::read(path)?;
    let original_path = managed.join(format!("{}-original", digest(&original)));
    if !original_path.exists() {
        std::fs::write(original_path, original)?;
    }

    let image = if width > MAX_ARTWORK_EDGE || height > MAX_ARTWORK_EDGE {
        image.thumbnail(MAX_ARTWORK_EDGE, MAX_ARTWORK_EDGE)
    } else {
        image
    };
    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(artwork_error)?;
    let bytes = buffer.into_inner();
    let output = managed.join(format!("{}.png", digest(&bytes)));
    std::fs::write(&output, &bytes)?;
    Ok(output.to_string_lossy().into_owned())
}

/// Writes the plan's metadata into a staged file, then reads it back to verify it.
pub fn apply(path: &Path, plan: &Plan, segment: &Segment, index: usize) -> Result<()> {
    if !TAGGABLE_EXTENSIONS.contains(&plan.extension.as_str()) {
        return Ok(());
    }
    let mut file = lofty::read_from_path(path).map_err(|e| AppError::new("TAGS", e))?;
    let tag_type = match plan.extension.as_str() {
        "mp3" | "wav" => TagType::Id3v2,
        "m4a" => TagType::Mp4Ilst,
        _ => TagType::VorbisComments,
    };
    if file.tag(tag_type).is_none() {
        file.insert_tag(Tag::new(tag_type));
    }
    let tag = file.tag_mut(tag_type).unwrap();

    if plan.outputs.len() > 1 {
        tag.insert_text(ItemKey::TrackTitle, segment.title.clone());
        tag.insert_text(ItemKey::TrackNumber, (index + 1).to_string());
        tag.insert_text(ItemKey::TrackTotal, plan.outputs.len().to_string());
    }
    apply_tag_edits(tag, plan)?;
    apply_artwork_edit(tag, plan)?;
    file.save_to_path(path, WriteOptions::default().use_id3v23(true))
        .map_err(|e| AppError::new("TAGS", e))?;

    let verified = lofty::read_from_path(path).map_err(|e| AppError::new("TAG_VERIFY", e))?;
    let tag = verified
        .tag(tag_type)
        .ok_or_else(|| AppError::new("TAG_VERIFY", "Written metadata could not be read back"))?;
    verify_tag_edits(tag, plan)?;
    verify_artwork_edit(tag, plan)
}

fn apply_tag_edits(tag: &mut Tag, plan: &Plan) -> Result<()> {
    for (name, edit) in &plan.request.tags {
        let Some(key) = key(name) else {
            continue;
        };
        match edit {
            Edit::Unchanged => {}
            Edit::Remove => {
                tag.remove_key(&key);
            }
            Edit::Set(value) => {
                if !tag.insert_text(key, value.clone()) {
                    return Err(AppError::new(
                        "TAG_UNSUPPORTED",
                        format!("This format cannot store {name}"),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn apply_artwork_edit(tag: &mut Tag, plan: &Plan) -> Result<()> {
    match &plan.request.artwork {
        Edit::Unchanged => {}
        Edit::Remove => remove_pictures(tag),
        Edit::Set(path) => {
            let mut reader = std::io::BufReader::new(std::fs::File::open(path)?);
            let mut picture = Picture::from_reader(&mut reader).map_err(artwork_error)?;
            picture.set_pic_type(PictureType::CoverFront);
            remove_pictures(tag);
            tag.push_picture(picture);
        }
    }
    Ok(())
}

fn remove_pictures(tag: &mut Tag) {
    while !tag.pictures().is_empty() {
        tag.remove_picture(0);
    }
}

fn verify_tag_edits(tag: &Tag, plan: &Plan) -> Result<()> {
    for (name, edit) in &plan.request.tags {
        let Some(key) = key(name) else {
            continue;
        };
        match edit {
            Edit::Set(value) if tag.get_string(&key) != Some(value.as_str()) => {
                return Err(AppError::new(
                    "TAG_VERIFY",
                    format!("{name} did not survive metadata read-back"),
                ))
            }
            Edit::Remove if tag.get_string(&key).is_some() => {
                return Err(AppError::new(
                    "TAG_VERIFY",
                    format!("{name} was not removed"),
                ))
            }
            _ => {}
        }
    }
    Ok(())
}

fn verify_artwork_edit(tag: &Tag, plan: &Plan) -> Result<()> {
    match &plan.request.artwork {
        Edit::Set(_) if tag.pictures().is_empty() => {
            Err(AppError::new("TAG_VERIFY", "Artwork was not retained"))
        }
        Edit::Remove if !tag.pictures().is_empty() => {
            Err(AppError::new("TAG_VERIFY", "Artwork was not removed"))
        }
        _ => Ok(()),
    }
}
