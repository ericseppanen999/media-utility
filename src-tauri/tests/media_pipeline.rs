use media_utility::{
    domain::*,
    jobs::Engine,
    media::{self, Tools},
    planner, process,
    storage::Storage,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_util::sync::CancellationToken;
fn tools() -> Tools {
    Tools {
        directory: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"),
    }
}
async fn fixture(tools: &Tools, dir: &Path) -> Source {
    let path = dir.join("Original Björk 'quoted'.wav");
    let args = [
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=3",
        "-c:a",
        "pcm_s16le",
    ]
    .map(str::to_string)
    .into_iter()
    .chain([path.to_string_lossy().into_owned()])
    .collect::<Vec<_>>();
    process::run(
        &tools.path("ffmpeg"),
        &args,
        &CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    media::inspect(tools, &path, &CancellationToken::new())
        .await
        .unwrap()
}
async fn finish(engine: &Engine, id: &str) -> Job {
    for _ in 0..600 {
        let job: Job = engine.db.get("job", id).await.unwrap();
        if !["queued", "running", "cancelling"].contains(&job.state.as_str()) {
            return job;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("Job timed out")
}
fn request(source: &Source, dest: &Path, preset: Preset) -> ExportRequest {
    ExportRequest {
        source_id: source.id.clone(),
        destination: dest.display().to_string(),
        preset,
        bitrate: 320,
        segments: vec![
            Segment {
                id: id(),
                start_us: 250_000,
                end_us: 1_500_000,
                title: "First / track".into(),
            },
            Segment {
                id: id(),
                start_us: 1_500_000,
                end_us: 2_750_000,
                title: "Second track".into(),
            },
        ],
        independent_clips: false,
        tags: BTreeMap::from([
            ("artist".into(), Edit::Set("Björk".into())),
            ("album".into(), Edit::Set("Live test".into())),
        ]),
        artwork: Edit::Unchanged,
        format_id: None,
    }
}
#[tokio::test]
#[ignore = "requires prepared media tools"]
async fn real_audio_exports_and_original_preservation() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools();
    let source = fixture(&tools, dir.path()).await;
    let original = std::fs::read(source.path.as_ref().unwrap()).unwrap();
    let db = Storage::open(&dir.path().join("test.sqlite")).unwrap();
    let engine = Arc::new(Engine::new(db, tools.clone(), dir.path(), Arc::new(|_, _| {})).unwrap());
    for preset in [Preset::Mp3, Preset::M4a, Preset::Flac, Preset::Wav] {
        let plan = planner::plan(
            source.clone(),
            request(&source, dir.path(), preset),
            BTreeMap::new(),
        )
        .unwrap();
        engine.db.put("plan", &plan.id, &plan).await.unwrap();
        let job = engine.enqueue(plan).await.unwrap();
        let done = finish(&engine, &job.id).await;
        assert_eq!(done.state, "succeeded", "{:?}", done.error);
        assert_eq!(done.artifacts.len(), 2);
        for a in &done.artifacts {
            assert!(Path::new(&a.path).is_file());
            let p = media::inspect(&tools, Path::new(&a.path), &CancellationToken::new())
                .await
                .unwrap();
            assert!((p.duration_us.unwrap() - 1_250_000).abs() < 250_000);
        }
    }
    assert_eq!(
        std::fs::read(source.path.as_ref().unwrap()).unwrap(),
        original
    );
}
#[tokio::test]
#[ignore = "requires prepared media tools"]
async fn cancellation_and_source_change_are_safe() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools();
    let source = fixture(&tools, dir.path()).await;
    let db = Storage::open(&dir.path().join("test.sqlite")).unwrap();
    let engine = Arc::new(Engine::new(db, tools, dir.path(), Arc::new(|_, _| {})).unwrap());
    let plan = planner::plan(
        source.clone(),
        request(&source, dir.path(), Preset::Mp3),
        BTreeMap::new(),
    )
    .unwrap();
    engine.db.put("plan", &plan.id, &plan).await.unwrap();
    let job = engine.enqueue(plan.clone()).await.unwrap();
    engine.cancel(&job.id).await.unwrap();
    assert_eq!(finish(&engine, &job.id).await.state, "cancelled");
    std::fs::write(source.path.as_ref().unwrap(), b"changed").unwrap();
    let job = engine.enqueue(plan).await.unwrap();
    let done = finish(&engine, &job.id).await;
    assert_eq!(done.state, "failed");
    assert_eq!(done.error.unwrap().code, "SOURCE_CHANGED");
}
#[tokio::test]
async fn interrupted_jobs_are_not_reported_as_successful() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.sqlite");
    let db = Storage::open(&path).unwrap();
    let job = Job {
        id: id(),
        plan_id: id(),
        title: "Recovery".into(),
        state: "running".into(),
        stage: "transform".into(),
        progress: Some(0.4),
        attempt: 1,
        sequence: 3,
        error: None,
        artifacts: vec![],
        created_at: now(),
        destination: dir.path().display().to_string(),
    };
    db.put("job", &job.id, &job).await.unwrap();
    drop(db);
    let recovered = Storage::open(&path)
        .unwrap()
        .get::<Job>("job", &job.id)
        .await
        .unwrap();
    assert_eq!(recovered.state, "interrupted");
    assert_eq!(recovered.error.unwrap().code, "INTERRUPTED");
}
