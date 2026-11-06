//! Runs export plans as background jobs: acquire, transcode, tag, validate, finalize.

use crate::{
    domain::*,
    downloads,
    media::{self, Tools},
    paths, process,
    storage::Storage,
    tags,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{Mutex, Semaphore, SemaphorePermit};
use tokio_util::sync::CancellationToken;

pub type Emit = Arc<dyn Fn(&str, &Job) + Send + Sync>;

const MAX_CONCURRENT_TRANSFORMS: usize = 1;
const MAX_CONCURRENT_DOWNLOADS: usize = 2;
const COPY_DURATION_TOLERANCE_US: i64 = 5_000_000;
const ENCODE_DURATION_TOLERANCE_US: i64 = 250_000;

pub struct Engine {
    pub db: Storage,
    pub tools: Tools,
    pub cache: PathBuf,
    pub artwork: PathBuf,
    active: Mutex<HashMap<String, CancellationToken>>,
    transform: Semaphore,
    acquire: Semaphore,
    pub emit: Emit,
}

/// Everything needed to produce one output file of a job.
struct OutputExport<'a> {
    plan: &'a Plan,
    output: &'a PlannedOutput,
    index: usize,
    input: &'a Path,
    input_probe: &'a Source,
    staged: &'a Path,
    destination: &'a Path,
    cancel: &'a CancellationToken,
}

impl Engine {
    pub fn new(db: Storage, tools: Tools, root: &Path, emit: Emit) -> Result<Self> {
        let cache = root.join("work");
        let artwork = root.join("artwork");
        std::fs::create_dir_all(&cache)?;
        std::fs::create_dir_all(&artwork)?;
        Ok(Self {
            db,
            tools,
            cache,
            artwork,
            active: Mutex::new(HashMap::new()),
            transform: Semaphore::new(MAX_CONCURRENT_TRANSFORMS),
            acquire: Semaphore::new(MAX_CONCURRENT_DOWNLOADS),
            emit,
        })
    }

    pub async fn enqueue(self: &Arc<Self>, plan: Plan) -> Result<Job> {
        let job = Job {
            id: id(),
            plan_id: plan.id.clone(),
            title: plan.source.title.clone(),
            state: "queued".into(),
            stage: "inspect".into(),
            progress: None,
            attempt: 1,
            sequence: 0,
            error: None,
            artifacts: vec![],
            created_at: now(),
            destination: plan.request.destination.clone(),
        };
        self.launch(job, plan).await
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        let active = self.active.lock().await;
        let cancel = active
            .get(id)
            .ok_or_else(|| AppError::new("JOB", "Job is no longer active"))?;
        cancel.cancel();
        Ok(())
    }

    pub async fn retry(self: &Arc<Self>, id: &str) -> Result<Job> {
        if self.active.lock().await.contains_key(id) {
            return Err(AppError::new("JOB", "This job is still active"));
        }
        let mut job: Job = self.db.get("job", id).await?;
        if !["failed", "cancelled", "interrupted"].contains(&job.state.as_str()) {
            return Err(AppError::new(
                "JOB",
                "Only failed, cancelled, or interrupted jobs can be retried",
            ));
        }
        let plan: Plan = self.db.get("plan", &job.plan_id).await?;
        remove_stale_staged_files(&job, &plan)?;
        job.state = "queued".into();
        job.attempt += 1;
        job.progress = None;
        job.error = None;
        self.launch(job, plan).await
    }

    pub async fn has_active(&self) -> bool {
        !self.active.lock().await.is_empty()
    }

    pub async fn cancel_all(&self) {
        for token in self.active.lock().await.values() {
            token.cancel()
        }
    }

    async fn publish(&self, job: &mut Job, event: &str) -> Result<()> {
        job.sequence += 1;
        self.db.put("job", &job.id, job).await?;
        (self.emit)(event, job);
        Ok(())
    }

    async fn enter_stage(&self, job: &mut Job, stage: &str) -> Result<()> {
        job.stage = stage.into();
        self.publish(job, "stage_progress").await
    }

    async fn launch(self: &Arc<Self>, mut job: Job, plan: Plan) -> Result<Job> {
        let cancel = CancellationToken::new();
        self.active
            .lock()
            .await
            .insert(job.id.clone(), cancel.clone());
        if let Err(e) = self.publish(&mut job, "job_state_changed").await {
            self.active.lock().await.remove(&job.id);
            return Err(e);
        }
        let initial = job.clone();
        let engine = self.clone();
        tokio::spawn(async move {
            let result = engine.execute(&mut job, &plan, &cancel).await;
            engine.finish(&mut job, result).await;
            engine.active.lock().await.remove(&job.id);
        });
        Ok(initial)
    }

    /// Records the terminal state of a job after `execute` returns.
    async fn finish(&self, job: &mut Job, result: Result<()>) {
        match result {
            Ok(()) => {
                job.state = "succeeded".into();
                job.stage = "finalize".into();
                job.progress = Some(1.0);
            }
            Err(e) if e.code == "CANCELLED" => {
                job.state = "cancelling".into();
                let _ = self.publish(job, "job_state_changed").await;
                job.state = "cancelled".into();
                job.error = Some(e);
            }
            Err(e) => {
                job.state = "failed".into();
                job.error = Some(e);
            }
        }
        let event = if job.error.is_some() {
            "job_error"
        } else {
            "job_state_changed"
        };
        let _ = self.publish(job, event).await;
    }

    async fn execute(&self, job: &mut Job, plan: &Plan, cancel: &CancellationToken) -> Result<()> {
        ensure_not_cancelled(cancel)?;
        job.state = "running".into();
        self.publish(job, "job_state_changed").await?;
        ensure_source_unchanged(plan)?;

        let work = self.cache.join(&job.id).join(job.attempt.to_string());
        std::fs::create_dir_all(&work)?;
        let input = self.resolve_input(job, plan, &work, cancel).await?;

        let _transform_slot = acquire_slot(&self.transform, cancel).await?;
        let input_probe = media::inspect(&self.tools, &input, cancel).await?;
        ensure_required_stream(plan, &input_probe)?;

        for (index, output) in plan.outputs.iter().enumerate() {
            ensure_not_cancelled(cancel)?;
            if self.artifact_is_intact(job, output, cancel).await? {
                continue;
            }
            let destination = Path::new(&output.path);
            let parent = destination
                .parent()
                .ok_or_else(|| AppError::new("DESTINATION", "Invalid destination"))?;
            let staged = parent.join(staged_file_name(job, job.attempt, index, plan));
            let export = OutputExport {
                plan,
                output,
                index,
                input: &input,
                input_probe: &input_probe,
                staged: &staged,
                destination,
                cancel,
            };
            let result = self.export_output(job, &export).await;
            if result.is_err() {
                let _ = std::fs::remove_file(&staged);
            }
            result?;
        }
        Ok(())
    }

    /// Returns the local media file to transform, downloading online sources when needed.
    async fn resolve_input(
        &self,
        job: &mut Job,
        plan: &Plan,
        work: &Path,
        cancel: &CancellationToken,
    ) -> Result<PathBuf> {
        if let Some(path) = &plan.source.path {
            return Ok(PathBuf::from(path));
        }
        self.enter_stage(job, "acquire").await?;
        let _download_slot = acquire_slot(&self.acquire, cancel).await?;
        if let Some(path) = self.reusable_download(plan, cancel).await {
            return Ok(path);
        }
        let path = downloads::acquire(&self.tools, plan, work, cancel).await?;
        media::inspect(&self.tools, &path, cancel).await?;
        let record = serde_json::json!({
            "path": path,
            "hash": paths::content_hash(&path)?,
        });
        self.db.put("download", &plan.hash, &record).await?;
        Ok(path)
    }

    /// Finds a previous download for this plan, if it is still complete and playable.
    async fn reusable_download(&self, plan: &Plan, cancel: &CancellationToken) -> Option<PathBuf> {
        let record = self
            .db
            .get::<serde_json::Value>("download", &plan.hash)
            .await
            .ok()?;
        let path = PathBuf::from(record["path"].as_str()?);
        let intact = path.starts_with(&self.cache)
            && path.is_file()
            && paths::content_hash(&path).ok().as_deref() == record["hash"].as_str()
            && media::inspect(&self.tools, &path, cancel).await.is_ok();
        intact.then_some(path)
    }

    /// Checks whether an earlier attempt already produced this output unchanged.
    async fn artifact_is_intact(
        &self,
        job: &Job,
        output: &PlannedOutput,
        cancel: &CancellationToken,
    ) -> Result<bool> {
        let existing = job
            .artifacts
            .iter()
            .find(|a| a.segment_id == output.segment.id);
        let Some(artifact) = existing else {
            return Ok(false);
        };
        let Ok(metadata) = std::fs::metadata(&artifact.path) else {
            return Ok(false);
        };
        let path = Path::new(&artifact.path);
        Ok(metadata.len() == artifact.size
            && paths::content_hash(path)? == artifact.content_hash
            && media::inspect(&self.tools, path, cancel).await.is_ok())
    }

    async fn export_output(&self, job: &mut Job, export: &OutputExport<'_>) -> Result<()> {
        self.transcode(job, export).await?;

        ensure_not_cancelled(export.cancel)?;
        job.progress = None;
        self.enter_stage(job, "tag").await?;
        apply_tags(export).await?;

        ensure_not_cancelled(export.cancel)?;
        self.enter_stage(job, "validate").await?;
        self.validate_staged(export).await?;

        ensure_not_cancelled(export.cancel)?;
        self.enter_stage(job, "finalize").await?;
        self.finalize_output(job, export).await
    }

    /// Runs FFmpeg for one output, publishing its progress as it goes.
    async fn transcode(&self, job: &mut Job, export: &OutputExport<'_>) -> Result<()> {
        job.progress = Some(0.0);
        self.enter_stage(job, "transform").await?;

        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<i64>(8);
        let callback: process::Progress = Arc::new(move |out_time_us| {
            let _ = progress_tx.try_send(out_time_us);
        });
        let segment = &export.output.segment;
        let args = media::ffmpeg_args(export.plan, export.input, segment, export.staged);
        let executable = self.tools.path("ffmpeg");
        let process = process::run(&executable, &args, export.cancel, Some(callback));
        tokio::pin!(process);

        loop {
            tokio::select! {
                result = &mut process => {
                    result?;
                    return Ok(());
                }
                Some(out_time_us) = progress_rx.recv() => {
                    let fraction = out_time_us as f64 / segment.duration_us() as f64;
                    job.progress = Some(fraction.clamp(0.0, 0.99));
                    self.publish(job, "stage_progress").await?;
                }
            }
        }
    }

    /// Confirms the staged file has the planned duration and, for copies, the source codecs.
    async fn validate_staged(&self, export: &OutputExport<'_>) -> Result<()> {
        let probe = media::inspect(&self.tools, export.staged, export.cancel).await?;
        let expected = export.output.segment.duration_us();
        let actual = probe
            .duration_us
            .ok_or_else(|| AppError::new("VALIDATION", "Output duration could not be verified"))?;
        let tolerance = if export.plan.copy {
            COPY_DURATION_TOLERANCE_US
        } else {
            ENCODE_DURATION_TOLERANCE_US
        };
        if (actual - expected).abs() > tolerance {
            return Err(AppError::new(
                "VALIDATION",
                "Output duration differs from the planned region",
            ));
        }
        if export.plan.copy {
            let source_streams = &export.input_probe.streams;
            for stream in &probe.streams {
                let matches_source = source_streams
                    .iter()
                    .any(|s| s.kind == stream.kind && s.codec == stream.codec);
                if !matches_source {
                    return Err(AppError::new(
                        "VALIDATION",
                        "Stream-copy output codec differs from the source",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Moves the staged file to its destination and records it as a job artifact.
    async fn finalize_output(&self, job: &mut Job, export: &OutputExport<'_>) -> Result<()> {
        std::fs::OpenOptions::new()
            .write(true)
            .open(export.staged)?
            .sync_all()?;
        let final_path = paths::finalize(export.staged, export.destination)?;
        let size = final_path.metadata()?.len();
        let content_hash = paths::content_hash(&final_path)?;
        let segment_id = &export.output.segment.id;
        job.artifacts.retain(|a| &a.segment_id != segment_id);
        job.artifacts.push(Artifact {
            id: id(),
            segment_id: segment_id.clone(),
            path: final_path.to_string_lossy().into_owned(),
            size,
            content_hash,
        });
        self.publish(job, "artifact_ready").await
    }
}

async fn apply_tags(export: &OutputExport<'_>) -> Result<()> {
    let path = export.staged.to_path_buf();
    let plan = export.plan.clone();
    let segment = export.output.segment.clone();
    let index = export.index;
    tokio::task::spawn_blocking(move || tags::apply(&path, &plan, &segment, index))
        .await
        .map_err(|_| AppError::new("TAGS", "Tag worker stopped"))?
}

async fn acquire_slot<'a>(
    slots: &'a Semaphore,
    cancel: &CancellationToken,
) -> Result<SemaphorePermit<'a>> {
    tokio::select! {
        permit = slots.acquire() => permit.map_err(|_| AppError::new("QUEUE", "Queue closed")),
        _ = cancel.cancelled() => Err(cancelled_error()),
    }
}

fn cancelled_error() -> AppError {
    AppError::new("CANCELLED", "Job cancelled")
}

fn ensure_not_cancelled(cancel: &CancellationToken) -> Result<()> {
    if cancel.is_cancelled() {
        Err(cancelled_error())
    } else {
        Ok(())
    }
}

fn ensure_source_unchanged(plan: &Plan) -> Result<()> {
    if let Some(path) = &plan.source.path {
        if paths::fingerprint(Path::new(path))? != plan.source.fingerprint {
            return Err(AppError::new(
                "SOURCE_CHANGED",
                "The input changed after planning. Inspect it again and create a new plan.",
            ));
        }
    }
    Ok(())
}

fn ensure_required_stream(plan: &Plan, input_probe: &Source) -> Result<()> {
    let required = if plan.request.preset.is_video() {
        "video"
    } else {
        "audio"
    };
    if input_probe.streams.iter().any(|s| s.kind == required) {
        Ok(())
    } else {
        Err(AppError::new(
            "STREAM",
            "Acquired source does not contain the required stream",
        ))
    }
}

fn staged_file_name(job: &Job, attempt: u32, index: usize, plan: &Plan) -> String {
    format!(
        ".media-utility-{}-{}-{}.{}",
        job.id, attempt, index, plan.extension
    )
}

/// Deletes staged files that earlier attempts of this job left next to its outputs.
fn remove_stale_staged_files(job: &Job, plan: &Plan) -> Result<()> {
    for (index, output) in plan.outputs.iter().enumerate() {
        let Some(parent) = Path::new(&output.path).parent() else {
            continue;
        };
        for attempt in 1..=job.attempt {
            let stale = parent.join(staged_file_name(job, attempt, index, plan));
            if stale.is_file() {
                std::fs::remove_file(stale)?;
            }
        }
    }
    Ok(())
}
