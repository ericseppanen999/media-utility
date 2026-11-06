use crate::{
    domain::*,
    downloads,
    jobs::{Emit, Engine},
    media::{self, Tools},
    paths, planner, process, segments,
    storage::Storage,
    tags,
};
use std::{path::Path, sync::Arc};
use tauri::{Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

type AppState = Arc<Engine>;

const MAX_PROJECT_BYTES: usize = 2 * 1024 * 1024;

async fn store_source(app: &tauri::AppHandle, state: &AppState, source: &Source) -> Result<()> {
    state.db.put("source", &source.id, source).await?;
    app.emit("source_ready", source)
        .map_err(|e| AppError::new("EVENT", e))
}

/// Normalizes artwork into the managed folder on a blocking worker.
async fn prepare_artwork(state: &AppState, path: String) -> Result<String> {
    let managed = state.artwork.clone();
    tokio::task::spawn_blocking(move || tags::prepare_artwork(Path::new(&path), &managed))
        .await
        .map_err(|_| AppError::new("ARTWORK", "Image worker failed"))?
}

#[tauri::command]
async fn inspect_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<Source> {
    let source = media::inspect(&state.tools, Path::new(&path), &CancellationToken::new()).await?;
    if let Some(path) = &source.path {
        app.asset_protocol_scope()
            .allow_file(path)
            .map_err(|e| AppError::new("PREVIEW", e))?;
    }
    store_source(&app, &state, &source).await?;
    Ok(source)
}

#[tauri::command]
async fn inspect_url(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
) -> Result<Source> {
    let source = downloads::inspect(&state.tools, &url).await?;
    store_source(&app, &state, &source).await?;
    Ok(source)
}

#[tauri::command]
fn parse_segments(text: String, duration_us: i64) -> Result<ParsedSegments> {
    segments::parse(&text, duration_us)
}

#[tauri::command]
async fn list_sources(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Vec<Source>> {
    let sources: Vec<Source> = state.db.list("source").await?;
    for source in &sources {
        if let Some(path) = &source.path {
            let _ = app.asset_protocol_scope().allow_file(path);
        }
    }
    Ok(sources)
}

#[tauri::command]
async fn save_project(state: State<'_, AppState>, project: serde_json::Value) -> Result<()> {
    let id = project["id"]
        .as_str()
        .filter(|id| id.len() < 100)
        .ok_or_else(|| AppError::new("PROJECT", "Project requires an ID"))?;
    if serde_json::to_vec(&project)?.len() > MAX_PROJECT_BYTES {
        return Err(AppError::new("PROJECT", "Project exceeds the size limit"));
    }
    state.db.put("project", id, &project).await
}

#[tauri::command]
async fn get_project(state: State<'_, AppState>, id: String) -> Result<Option<serde_json::Value>> {
    let all: Vec<serde_json::Value> = state.db.list("project").await?;
    Ok(all.into_iter().find(|p| p["id"].as_str() == Some(&id)))
}

#[tauri::command]
async fn plan_export(state: State<'_, AppState>, mut request: ExportRequest) -> Result<Plan> {
    let source: Source = state.db.get("source", &request.source_id).await?;
    if let Edit::Set(path) = &request.artwork {
        let stored = prepare_artwork(&state, path.clone()).await?;
        request.artwork = Edit::Set(stored);
    }
    let plan = planner::plan(source, request, state.tools.versions().await)?;
    state.db.put("plan", &plan.id, &plan).await?;
    Ok(plan)
}

#[tauri::command]
async fn enqueue_job(
    state: State<'_, AppState>,
    plan_id: String,
    plan_hash: String,
) -> Result<Job> {
    let plan: Plan = state.db.get("plan", &plan_id).await?;
    if plan.hash != plan_hash {
        return Err(AppError::new(
            "PLAN_CHANGED",
            "The plan changed; review a new plan before exporting",
        ));
    }
    state.inner().enqueue(plan).await
}

#[tauri::command]
async fn cancel_job(state: State<'_, AppState>, job_id: String) -> Result<()> {
    state.cancel(&job_id).await
}

#[tauri::command]
async fn retry_job(state: State<'_, AppState>, job_id: String) -> Result<Job> {
    state.inner().retry(&job_id).await
}

#[tauri::command]
async fn list_jobs(state: State<'_, AppState>) -> Result<Vec<Job>> {
    state.db.list("job").await
}

#[tauri::command]
async fn get_artifacts(state: State<'_, AppState>, job_id: String) -> Result<Vec<Artifact>> {
    Ok(state.db.get::<Job>("job", &job_id).await?.artifacts)
}

#[tauri::command]
async fn get_diagnostics(state: State<'_, AppState>) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "app_version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "tools": state.tools.versions().await,
        "tool_directory": state.tools.directory,
        "database": "SQLite · WAL · schema 1",
    }))
}

#[tauri::command]
async fn reveal_output(
    state: State<'_, AppState>,
    job_id: String,
    artifact_id: String,
) -> Result<()> {
    let job: Job = state.db.get("job", &job_id).await?;
    let artifact = job
        .artifacts
        .iter()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| AppError::new("ARTIFACT", "Unknown output"))?;
    if !Path::new(&artifact.path).is_file() {
        return Err(AppError::new(
            "ARTIFACT",
            "The output has been moved or deleted",
        ));
    }

    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", artifact.path))
            .spawn()?;
    }
    Ok(())
}

#[tauri::command]
async fn shutdown_jobs(state: State<'_, AppState>) -> Result<()> {
    state.cancel_all().await;
    for _ in 0..300 {
        if !state.has_active().await {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    Err(AppError::new(
        "SHUTDOWN",
        "Still stopping a media worker. Wait and try closing again.",
    ))
}

#[tauri::command]
async fn import_artwork(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String> {
    let output = prepare_artwork(&state, path).await?;
    app.asset_protocol_scope()
        .allow_file(&output)
        .map_err(|e| AppError::new("ARTWORK", e))?;
    Ok(output)
}

#[tauri::command]
async fn create_preview(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    source_id: String,
) -> Result<String> {
    let source: Source = state.db.get("source", &source_id).await?;
    let input = source
        .path
        .as_ref()
        .ok_or_else(|| AppError::new("PREVIEW", "Online previews are available after download"))?;
    if paths::fingerprint(Path::new(input))? != source.fingerprint {
        return Err(AppError::new(
            "SOURCE_CHANGED",
            "Re-import the changed source before previewing",
        ));
    }
    let folder = state.cache.join("previews");
    std::fs::create_dir_all(&folder)?;
    let path = folder.join(format!("{}.mp4", source.fingerprint));
    if !path.exists() {
        let staged = folder.join(format!("{}.mp4", id()));
        let has_video = source.streams.iter().any(|s| s.kind == "video");
        let args = media::preview_args(input, has_video, &staged);
        let result = process::run(
            &state.tools.path("ffmpeg"),
            &args,
            &CancellationToken::new(),
            None,
        )
        .await;
        if let Err(e) = result {
            let _ = std::fs::remove_file(&staged);
            return Err(e);
        }
        if path.exists() {
            std::fs::remove_file(staged)?;
        } else {
            std::fs::rename(staged, &path)?;
        }
    }
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|e| AppError::new("PREVIEW", e))?;
    Ok(path.to_string_lossy().into_owned())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            std::fs::create_dir_all(&root)?;
            app.asset_protocol_scope()
                .allow_directory(root.join("artwork"), false)?;
            let tools_dir = if cfg!(debug_assertions) {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries")
            } else {
                app.path().resource_dir()?.join("binaries")
            };
            let handle = app.handle().clone();
            let emit: Emit = Arc::new(move |event, job| {
                let _ = handle.emit(event, job);
            });
            let db = Storage::open(&root.join("media-utility.sqlite"))
                .map_err(|e| std::io::Error::other(e.message))?;
            app.manage(Arc::new(
                Engine::new(
                    db,
                    Tools {
                        directory: tools_dir,
                    },
                    &root,
                    emit,
                )
                .map_err(|e| std::io::Error::other(e.message))?,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            inspect_file,
            inspect_url,
            parse_segments,
            list_sources,
            save_project,
            get_project,
            plan_export,
            enqueue_job,
            cancel_job,
            retry_job,
            list_jobs,
            get_artifacts,
            get_diagnostics,
            reveal_output,
            shutdown_jobs,
            import_artwork,
            create_preview
        ])
        .run(tauri::generate_context!())
        .expect("could not start Media Utility");
}
