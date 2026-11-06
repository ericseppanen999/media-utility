use crate::{domain::*, media::Tools, process};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;

pub fn normalize_url(input: &str) -> Result<String> {
    let mut url =
        url::Url::parse(input).map_err(|_| AppError::new("URL", "Enter a valid YouTube URL"))?;
    if url.scheme() != "https"
        || !matches!(
            url.host_str(),
            Some("youtube.com" | "www.youtube.com" | "m.youtube.com" | "youtu.be")
        )
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::new(
            "URL",
            "P0 supports public HTTPS YouTube links",
        ));
    }
    let video = url
        .query_pairs()
        .find(|(k, _)| k == "v")
        .map(|(_, v)| v.to_string());
    if url.path() == "/playlist" || (url.path() == "/watch" && video.is_none()) {
        return Err(AppError::new(
            "PLAYLIST",
            "Choose a single video link. Playlist import is not supported yet.",
        ));
    }
    // Retain only the stable video identity, never signed parameters or secrets.
    url.set_query(None);
    url.set_fragment(None);
    if let Some(v) = video {
        url.query_pairs_mut().append_pair("v", &v);
    }
    Ok(url.to_string())
}

fn common(tools: &Tools) -> Vec<String> {
    vec![
        "--ignore-config".into(),
        "--no-playlist".into(),
        "--no-warnings".into(),
        "--ffmpeg-location".into(),
        tools.directory.to_string_lossy().into_owned(),
        "--js-runtimes".into(),
        format!("deno:{}", tools.path("deno").display()),
    ]
}

pub async fn inspect(tools: &Tools, url: &str) -> Result<Source> {
    let url = normalize_url(url)?;
    let mut args = common(tools);
    args.extend([
        "--dump-single-json".into(),
        "--skip-download".into(),
        "--".into(),
        url.clone(),
    ]);
    let output = process::run(
        &tools.path("yt-dlp"),
        &args,
        &CancellationToken::new(),
        None,
    )
    .await?;
    let info: Value = serde_json::from_str(&output)?;

    let duration_us = info["duration"]
        .as_f64()
        .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
        .map(|seconds| (seconds * 1e6) as i64);
    let formats = parse_formats(&info);
    let mut streams = vec![];
    if formats.iter().any(|f| f.audio) {
        streams.push(pending_stream(0, "audio"));
    }
    if formats.iter().any(|f| f.video) {
        streams.push(pending_stream(1, "video"));
    }
    Ok(Source {
        id: id(),
        title: info["title"].as_str().unwrap_or("Online media").into(),
        path: None,
        url: Some(url.clone()),
        duration_us,
        fingerprint: digest(format!("{}:{}", url, info["id"]).as_bytes()),
        streams,
        chapters: parse_chapters(&info),
        tags: BTreeMap::new(),
        formats,
        thumbnail: None,
    })
}

fn parse_formats(info: &Value) -> Vec<Format> {
    info["formats"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|f| {
            let id = f["format_id"].as_str()?.to_string();
            let audio = f["acodec"].as_str().is_some_and(|c| c != "none");
            let video = f["vcodec"].as_str().is_some_and(|c| c != "none");
            if !audio && !video {
                return None;
            }
            let note = f["format_note"].as_str().unwrap_or("original");
            let extension = f["ext"].as_str().unwrap_or("");
            Some(Format {
                label: format!("{id} · {note} · {extension}"),
                id,
                audio,
                video,
            })
        })
        .collect()
}

fn parse_chapters(info: &Value) -> Vec<Segment> {
    info["chapters"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|c| {
            Some(Segment {
                id: id(),
                start_us: (c["start_time"].as_f64()? * 1e6) as i64,
                end_us: (c["end_time"].as_f64()? * 1e6) as i64,
                title: c["title"].as_str().unwrap_or("Chapter").into(),
            })
        })
        .collect()
}

/// A stream whose codec is unknown until the media has been downloaded.
fn pending_stream(index: i64, kind: &str) -> Stream {
    Stream {
        index,
        kind: kind.into(),
        codec: "pending".into(),
        sample_rate: None,
        channels: None,
        width: None,
        height: None,
    }
}

pub async fn acquire(
    tools: &Tools,
    plan: &Plan,
    work: &Path,
    cancel: &CancellationToken,
) -> Result<PathBuf> {
    let output = work.join("source.%(ext)s");
    let selector = plan.request.format_id.clone().unwrap_or_else(|| {
        if plan.request.preset.is_video() {
            "bv*+ba/b".into()
        } else {
            "ba/b".into()
        }
    });
    let mut args = common(tools);
    args.extend([
        "--no-progress".into(),
        "--no-simulate".into(),
        "--print".into(),
        "after_move:filepath".into(),
        "-f".into(),
        selector,
        "--merge-output-format".into(),
        "mkv".into(),
        "-o".into(),
        output.to_string_lossy().into_owned(),
        "--".into(),
        plan.source.url.clone().unwrap(),
    ]);
    let reported = process::run(&tools.path("yt-dlp"), &args, cancel, None).await?;
    let path = PathBuf::from(
        reported
            .lines()
            .last()
            .ok_or_else(|| AppError::new("DOWNLOAD", "Extractor did not report an output path"))?,
    )
    .canonicalize()?;
    if !path.starts_with(work.canonicalize()?) {
        return Err(AppError::new(
            "DOWNLOAD",
            "Extractor returned a path outside its working directory",
        ));
    }
    Ok(path)
}
