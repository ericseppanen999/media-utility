//! Bundled tool lookup, ffprobe inspection, and FFmpeg argument construction.

use crate::{
    domain::*,
    paths,
    process::{self, strings},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Tools {
    pub directory: PathBuf,
}

impl Tools {
    pub fn path(&self, name: &str) -> PathBuf {
        self.directory
            .join(format!("{name}{}", if cfg!(windows) { ".exe" } else { "" }))
    }

    pub async fn versions(&self) -> BTreeMap<String, String> {
        let mut versions = BTreeMap::new();
        for name in ["ffmpeg", "ffprobe", "yt-dlp", "deno"] {
            let arg = if name == "ffmpeg" || name == "ffprobe" {
                "-version"
            } else {
                "--version"
            };
            let cancel = CancellationToken::new();
            let value = process::run(&self.path(name), &[arg.into()], &cancel, None)
                .await
                .map(|v| v.lines().next().unwrap_or("unknown").to_string())
                .unwrap_or_else(|_| "Missing or unavailable".into());
            versions.insert(name.into(), value);
        }
        versions
    }
}

pub async fn inspect(tools: &Tools, path: &Path, cancel: &CancellationToken) -> Result<Source> {
    let path = path.canonicalize()?;
    let mut args = strings(&[
        "-v",
        "error",
        "-show_format",
        "-show_streams",
        "-show_chapters",
        "-of",
        "json",
    ]);
    args.push(path_arg(&path));
    let output = process::run(&tools.path("ffprobe"), &args, cancel, None).await?;
    let probe: Value = serde_json::from_str(&output)?;

    let streams = parse_streams(&probe);
    if !streams
        .iter()
        .any(|s| s.kind == "audio" || s.kind == "video")
    {
        return Err(AppError::new(
            "MEDIA",
            "No playable audio or video streams found",
        ));
    }
    Ok(Source {
        id: id(),
        title: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        path: Some(path.to_string_lossy().into()),
        url: None,
        duration_us: parse_duration(&probe),
        fingerprint: paths::fingerprint(&path)?,
        streams,
        chapters: parse_chapters(&probe),
        tags: parse_tags(&probe),
        formats: vec![],
        thumbnail: None,
    })
}

fn parse_streams(probe: &Value) -> Vec<Stream> {
    probe["streams"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|s| s["disposition"]["attached_pic"].as_i64() != Some(1))
        .map(|s| Stream {
            index: s["index"].as_i64().unwrap_or(0),
            kind: s["codec_type"].as_str().unwrap_or("unknown").into(),
            codec: s["codec_name"].as_str().unwrap_or("unknown").into(),
            sample_rate: s["sample_rate"].as_str().map(str::to_string),
            channels: s["channels"].as_u64(),
            width: s["width"].as_u64(),
            height: s["height"].as_u64(),
        })
        .collect()
}

fn parse_duration(probe: &Value) -> Option<i64> {
    probe["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
        .map(|seconds| (seconds * 1e6) as i64)
}

fn parse_tags(probe: &Value) -> BTreeMap<String, String> {
    probe["format"]["tags"]
        .as_object()
        .map(|tags| {
            tags.iter()
                .filter_map(|(k, v)| v.as_str().map(|v| (k.to_lowercase(), v.into())))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_chapters(probe: &Value) -> Vec<Segment> {
    probe["chapters"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|c| {
            Some(Segment {
                id: id(),
                start_us: (c["start_time"].as_str()?.parse::<f64>().ok()? * 1e6) as i64,
                end_us: (c["end_time"].as_str()?.parse::<f64>().ok()? * 1e6) as i64,
                title: c["tags"]["title"].as_str().unwrap_or("Chapter").into(),
            })
        })
        .collect()
}

pub fn ffmpeg_args(plan: &Plan, input: &Path, segment: &Segment, staged: &Path) -> Vec<String> {
    let preset = &plan.request.preset;
    let video = preset.is_video();

    let mut args = strings(&["-hide_banner", "-nostdin", "-n", "-i"]);
    args.push(path_arg(input));
    args.extend([
        "-ss".into(),
        format!("{:.6}", segment.start_us as f64 / 1e6),
        "-t".into(),
        format!("{:.6}", segment.duration_us() as f64 / 1e6),
    ]);
    if video {
        args.extend(strings(&["-map", "0:v:0", "-map", "0:a:0?"]));
    } else {
        args.extend(strings(&["-map", "0:a:0", "-vn"]));
    }
    args.extend(strings(&["-map_chapters", "-1", "-map_metadata", "0"]));

    if plan.copy {
        args.extend(strings(&["-c", "copy"]));
    } else {
        let audio_codec = match preset {
            Preset::Mp3 => "libmp3lame",
            Preset::M4a | Preset::Mp4 => "aac",
            Preset::Flac => "flac",
            Preset::Wav => "pcm_s16le",
            _ => "copy",
        };
        args.extend(strings(&["-c:a", audio_codec]));
        if matches!(preset, Preset::Mp3 | Preset::M4a | Preset::Mp4) {
            args.extend(["-b:a".into(), format!("{}k", plan.request.bitrate)]);
        }
        if video {
            args.extend(strings(&[
                "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p",
            ]));
        }
    }

    args.extend(strings(&["-progress", "pipe:1", "-nostats"]));
    args.push(path_arg(staged));
    args
}

/// Arguments for a small MP4 that the webview can play when it cannot decode the original.
pub fn preview_args(input: &str, has_video: bool, staged: &Path) -> Vec<String> {
    let mut args = strings(&["-hide_banner", "-nostdin", "-n", "-i", input]);
    args.extend(strings(&["-map", "0:a:0?", "-c:a", "aac", "-b:a", "128k"]));
    if has_video {
        args.extend(strings(&[
            "-map",
            "0:v:0",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-vf",
            "scale=640:-2",
            "-pix_fmt",
            "yuv420p",
        ]));
    }
    args.extend(strings(&["-movflags", "+faststart"]));
    args.push(path_arg(staged));
    args
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
