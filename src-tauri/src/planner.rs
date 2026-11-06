//! Turns an export request into a reviewed, immutable plan.

use crate::{domain::*, paths, segments, tags::TAGGABLE_EXTENSIONS};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const SUPPORTED_BITRATES: [u32; 6] = [96, 128, 160, 192, 256, 320];
const TAG_FIELDS: [&str; 11] = [
    "title",
    "artist",
    "album_artist",
    "album",
    "track",
    "track_total",
    "disc",
    "disc_total",
    "year",
    "genre",
    "comment",
];

pub fn plan(
    source: Source,
    request: ExportRequest,
    dependencies: BTreeMap<String, String>,
) -> Result<Plan> {
    let duration = source.duration_us.ok_or_else(|| {
        AppError::new(
            "DURATION",
            "This source has no reliable duration; export is unavailable",
        )
    })?;
    segments::validate(&request.segments, duration, request.independent_clips)?;
    if !SUPPORTED_BITRATES.contains(&request.bitrate) {
        return Err(AppError::new("BITRATE", "Choose a supported bitrate"));
    }
    let destination = Path::new(&request.destination).canonicalize()?;
    if !destination.is_dir() {
        return Err(AppError::new(
            "DESTINATION",
            "Choose an existing output folder",
        ));
    }

    let stream = primary_stream(&source, &request.preset)?;
    validate_format(&source, &request)?;
    let trimmed = is_trimmed(&request.segments, duration);
    let (extension, copy) = choose_container(&source, stream, &request.preset, trimmed);
    validate_tags(&request, extension)?;

    let warnings = plan_warnings(&source, &request.preset, extension, copy, trimmed);
    let outputs = planned_outputs(&request.segments, &destination, extension);
    let explanation = if copy {
        format!("Copy original streams into .{extension}. No re-encoding.")
    } else {
        format!(
            "Encode directly from the original into .{extension}. \
             Lossy sources cannot gain detail."
        )
    };
    let mut plan = Plan {
        id: id(),
        hash: String::new(),
        source,
        request,
        outputs,
        extension: extension.into(),
        copy,
        explanation,
        warnings,
        dependencies,
    };
    plan.hash = digest(&serde_json::to_vec(&plan)?);
    Ok(plan)
}

/// Finds the stream that decides the output: video for video presets, otherwise audio.
fn primary_stream<'a>(source: &'a Source, preset: &Preset) -> Result<&'a Stream> {
    let (kind, missing) = if preset.is_video() {
        ("video", "Source has no video")
    } else {
        ("audio", "Source has no audio")
    };
    source
        .streams
        .iter()
        .find(|s| s.kind == kind)
        .ok_or_else(|| AppError::new("STREAM", missing))
}

fn validate_format(source: &Source, request: &ExportRequest) -> Result<()> {
    let Some(format_id) = &request.format_id else {
        return Ok(());
    };
    let format = source
        .formats
        .iter()
        .find(|f| &f.id == format_id)
        .ok_or_else(|| AppError::new("FORMAT", "Selected format is no longer available"))?;
    let usable = if request.preset.is_video() {
        format.video && format.audio
    } else {
        format.audio
    };
    if usable {
        Ok(())
    } else {
        Err(AppError::new(
            "FORMAT",
            "Choose a combined audio/video format or use Best available",
        ))
    }
}

/// True unless the request is a single region covering the whole source.
fn is_trimmed(segments: &[Segment], duration: i64) -> bool {
    segments.len() != 1 || segments[0].start_us != 0 || segments[0].end_us != duration
}

/// Picks the output extension and whether streams can be copied without re-encoding.
fn choose_container(
    source: &Source,
    stream: &Stream,
    preset: &Preset,
    trimmed: bool,
) -> (&'static str, bool) {
    match preset {
        Preset::OriginalAudio => (original_audio_extension(&stream.codec), true),
        Preset::OriginalVideo => ("mkv", true),
        Preset::Mp3 => ("mp3", false),
        Preset::M4a => ("m4a", stream.codec == "aac" && !trimmed),
        Preset::Flac => ("flac", false),
        Preset::Wav => ("wav", false),
        Preset::Mp4 => {
            let all_audio_aac = source
                .streams
                .iter()
                .filter(|s| s.kind == "audio")
                .all(|s| s.codec == "aac");
            ("mp4", !trimmed && stream.codec == "h264" && all_audio_aac)
        }
    }
}

fn original_audio_extension(codec: &str) -> &'static str {
    match codec {
        "aac" => "m4a",
        "mp3" => "mp3",
        "flac" => "flac",
        "opus" | "vorbis" => "ogg",
        c if c.starts_with("pcm_") => "wav",
        _ => "mka",
    }
}

fn validate_tags(request: &ExportRequest, extension: &str) -> Result<()> {
    let has_edits = request.tags.values().any(|v| !matches!(v, Edit::Unchanged))
        || !matches!(request.artwork, Edit::Unchanged);
    if has_edits && !TAGGABLE_EXTENSIONS.contains(&extension) {
        return Err(AppError::new(
            "TAGS_UNSUPPORTED",
            "This container does not have a verified music tag adapter. \
             Choose MP3, M4A, FLAC, WAV, or compatible original audio.",
        ));
    }
    if request
        .tags
        .keys()
        .any(|key| !TAG_FIELDS.contains(&key.as_str()))
    {
        return Err(AppError::new("TAG", "Unsupported metadata field"));
    }
    Ok(())
}

fn plan_warnings(
    source: &Source,
    preset: &Preset,
    extension: &str,
    copy: bool,
    trimmed: bool,
) -> Vec<String> {
    let mut warnings = vec![];
    if copy && trimmed {
        warnings.push(
            "Stream-copy cuts may move to packet or keyframe boundaries. \
             Choose an encoding preset for accurate cuts."
                .into(),
        );
    }
    if matches!(preset, Preset::Flac | Preset::Wav) {
        warnings.push(
            "Lossless output preserves decoded audio; it cannot restore detail lost in the source."
                .into(),
        );
    }
    if extension == "wav" {
        warnings.push("WAV tag and artwork playback support varies between players.".into());
    }
    if source.url.is_some() {
        warnings.push(
            "The full source is acquired once for this job; download size may be unknown.".into(),
        );
    }
    warnings
}

fn planned_outputs(
    segments: &[Segment],
    destination: &Path,
    extension: &str,
) -> Vec<PlannedOutput> {
    segments
        .iter()
        .enumerate()
        .map(|(i, segment)| {
            let title = paths::sanitize(&segment.title);
            let name = format!("{:02} - {title}.{extension}", i + 1);
            let path: PathBuf = paths::available(&destination.join(name));
            PlannedOutput {
                segment: segment.clone(),
                path: path.to_string_lossy().into_owned(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Source {
        Source {
            id: id(),
            title: "test".into(),
            path: None,
            url: None,
            duration_us: Some(1_000_000),
            fingerprint: "a".into(),
            streams: vec![Stream {
                index: 0,
                kind: "audio".into(),
                codec: "aac".into(),
                sample_rate: None,
                channels: None,
                width: None,
                height: None,
            }],
            chapters: vec![],
            tags: BTreeMap::new(),
            formats: vec![],
            thumbnail: None,
        }
    }

    #[test]
    fn copy_only_when_compatible() {
        let d = tempfile::tempdir().unwrap();
        let s = source();
        let mut r = ExportRequest {
            source_id: s.id.clone(),
            destination: d.path().display().to_string(),
            preset: Preset::M4a,
            bitrate: 320,
            segments: vec![Segment {
                id: id(),
                start_us: 0,
                end_us: 1_000_000,
                title: "Song".into(),
            }],
            independent_clips: false,
            tags: BTreeMap::new(),
            artwork: Edit::Unchanged,
            format_id: None,
        };
        assert!(plan(s.clone(), r.clone(), BTreeMap::new()).unwrap().copy);
        r.segments[0].start_us = 1;
        assert!(!plan(s, r, BTreeMap::new()).unwrap().copy)
    }
}
