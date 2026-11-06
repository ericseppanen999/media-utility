use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: &str, message: impl ToString) -> Self {
        Self {
            code: code.into(),
            message: message.to_string(),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new("IO", e)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::new("DATABASE", e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new("DATA", e)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stream {
    pub index: i64,
    pub kind: String,
    pub codec: String,
    pub sample_rate: Option<String>,
    pub channels: Option<u64>,
    pub width: Option<u64>,
    pub height: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Format {
    pub id: String,
    pub label: String,
    pub audio: bool,
    pub video: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub path: Option<String>,
    pub url: Option<String>,
    pub duration_us: Option<i64>,
    pub fingerprint: String,
    pub streams: Vec<Stream>,
    pub chapters: Vec<Segment>,
    pub tags: BTreeMap<String, String>,
    pub formats: Vec<Format>,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub start_us: i64,
    pub end_us: i64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSegments {
    pub segments: Vec<Segment>,
    pub unused_intro_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    OriginalAudio,
    Mp3,
    M4a,
    Flac,
    Wav,
    OriginalVideo,
    Mp4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "snake_case")]
pub enum Edit {
    Unchanged,
    Set(String),
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub source_id: String,
    pub destination: String,
    pub preset: Preset,
    pub bitrate: u32,
    pub segments: Vec<Segment>,
    pub independent_clips: bool,
    pub tags: BTreeMap<String, Edit>,
    pub artwork: Edit,
    pub format_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedOutput {
    pub segment: Segment,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub hash: String,
    pub source: Source,
    pub request: ExportRequest,
    pub outputs: Vec<PlannedOutput>,
    pub extension: String,
    pub copy: bool,
    pub explanation: String,
    pub warnings: Vec<String>,
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub segment_id: String,
    pub path: String,
    pub size: u64,
    #[serde(default)]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub plan_id: String,
    pub title: String,
    pub state: String,
    pub stage: String,
    pub progress: Option<f64>,
    pub attempt: u32,
    pub sequence: u64,
    pub error: Option<AppError>,
    pub artifacts: Vec<Artifact>,
    pub created_at: u64,
    pub destination: String,
}

impl Segment {
    pub fn duration_us(&self) -> i64 {
        self.end_us - self.start_us
    }
}

impl Preset {
    pub fn is_video(&self) -> bool {
        matches!(self, Preset::Mp4 | Preset::OriginalVideo)
    }
}

pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
