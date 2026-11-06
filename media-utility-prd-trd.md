# Media Utility — PRD and TRD

Version: 1.1 · Status: proposed build specification

Working product name: Media Utility. Naming and branding are outside this specification.

# Part I — Product Requirements Document

## 1. Product definition

A local desktop application for downloading online audio/video, converting formats, trimming and splitting media, and editing metadata and cover art. Users complete these operations through one interface without installing or operating command-line tools.

The primary product is a general media utility. Turning a concert into tagged tracks is one workflow, alongside downloading a video, extracting audio from a local recording, converting a file, or updating artwork across an existing album.

## 2. Problem and audience

Today, users move between a downloader, converter, editor, and tag manager. They encounter unfamiliar containers and codecs, lose quality through unnecessary conversions, and manually repeat tagging and naming tasks.

Initial audience: individual Windows users who manage personal media files and want clear controls without professional editing software. Windows 11 x64 is the first supported platform. macOS and Linux follow after the Windows workflow is stable.

## 3. Goals and success criteria

| Goal | Release acceptance criterion |
|---|---|
| Complete common tasks in one app | Download, convert, split, tag, and export without another application |
| No manual dependency setup | Clean Windows installation can complete a local-file export without Python, FFmpeg, or yt-dlp on PATH |
| Explain quality choices | Every export identifies whether streams are copied or re-encoded, with the actual output format |
| Protect original files | Default operations create new outputs; failed jobs never replace originals |
| Make batch work predictable | Each job shows its current stage, result, destination, and actionable failure |
| Produce portable music files | MP3 exports include readable titles, artist, album, track numbers, and optional embedded artwork |

Usability target: a new user can import a file and export a trimmed MP3 within two minutes, excluding processing time. Validate with five observed users before calling the UI release-ready. This is a proposed target, not a measured result.

## 4. Scope and priority

| Priority | Capability | Details |
|---|---|---|
| P0 | URL import | YouTube audio/video; inspect title, thumbnail, duration, and available formats |
| P0 | Local import | Drag/drop or file picker; inspect actual streams rather than trusting extensions |
| P0 | Download controls | Best available audio/video, explicit format selection, destination, original-stream download |
| P0 | Audio export | MP3, AAC in M4A, FLAC, WAV; original audio extraction where supported |
| P0 | Video export | Original streams in a compatible container; MP4 H.264/AAC compatibility preset |
| P0 | Trim and split | Start/end trim; multiple named regions; pasted timestamp/title list; chapter import |
| P0 | Metadata | Individual and bulk edits for title, artist, album artist, album, track/total, disc/total, year, genre, comment |
| P0 | Artwork | Add, replace, remove, preview; apply one image to multiple tracks |
| P0 | Queue and recovery | Progress, cancel, retry failed job, reveal output, persisted history |
| P0 | Export naming | Numbered names, sanitized paths, collision handling, preview before execution |
| P1 | Playlist downloads | Preview items and select a subset before queueing |
| P1 | Join media | Ordered concatenation with compatibility validation and explicit conversion when needed |
| P1 | Waveform selection | Zoom, playback, draggable region boundaries, fades |
| P1 | Convenience | Saved presets, metadata templates, thumbnail as cover, automatic silence suggestions |
| P2 | Broader platform support | More yt-dlp sites on a best-effort basis; macOS/Linux installers |

P0 editing is numeric trim/region editing with a playable preview. A multitrack timeline is not required to ship.

## 5. Main workflows

### Download a video or audio file

Paste URL → inspect source → choose audio/video and quality preset → select destination → preview output summary → queue → reveal file.

If a URL includes a timestamp, default to the full item and offer “start at linked time.” Playlist links require an explicit single-item versus playlist choice. Never silently download an entire playlist.

### Split a long recording

Import URL or local media → paste timestamps/titles or import chapters → validate and preview regions → edit shared artist/album/artwork → choose output preset → export named tracks.

Accept `M:SS Title` and `H:MM:SS Title`, one entry per line, including common copied YouTube timestamp links. Each start marks the next track boundary; the last track ends at the source duration. Explicit end times are editable. If the first start is later than zero, show the unused introduction. Overlaps are rejected in sequential split mode; independently selected clips may overlap.

### Edit an existing album

Import multiple files → select all or a subset → change shared tags/artwork → review changes → export updated copies. Bulk fields display “Mixed” and remain unchanged unless explicitly edited. Blank and remove are separate actions. Optional overwrite requires an explicit choice and a backup.

### Convert or trim a local file

Import → choose region if needed → choose destination format → inspect copy/re-encode explanation → export. Local processing remains available offline.

## 6. Interface requirements

Three main areas: Sources, Editor, and Jobs. Sources contains URL/file import and selected media details. Editor exposes Preview, Segments, Tags & Artwork, and Export. Jobs shows a queue and recent results.

Use human-readable presets alongside an optional technical details panel. Examples: “Keep original quality,” “MP3 — broad compatibility,” and “MP4 — broad compatibility.” Never label a lossy transcode “lossless” or imply that a higher bitrate restores missing detail. Container changes are distinguished from codec changes.

Keep source metadata as suggestions; never overwrite user-entered values when extraction refreshes. Uploader/channel is not automatically treated as the musical artist. Let users preview the expected filenames and total output count before queueing.

## 7. Quality and compatibility rules

- Keep original quality copies compressed streams where the target container permits it.
- Audio conversion encodes from the retained original once; split exports do not first convert a full intermediate album to MP3.
- MP3 defaults to 320 kbps for the high-quality compatibility preset; other bitrates are selectable.
- FLAC/WAV preserve decoded audio but cannot recover information lost in the source.
- Exact trimming may require re-encoding; stream-copy cuts can differ at packet/frame or video keyframe boundaries. Show the tradeoff before export.
- Unsupported tags/artwork are reported per format. Never claim a save succeeded if requested fields were silently discarded.
- Outputs are ordinary local files. Direct publishing to Spotify, cloud upload, accounts, and DRM-protected content are outside P0.

## 8. Non-goals

Professional multitrack editing, effects chains, AI mastering, transcription, cloud processing, social publishing, streaming-platform library management, and automatic music identification. These must not delay the initial utility workflows.

## 9. Failure behavior

Invalid timestamps identify the line and reason. An inaccessible URL explains whether retry, dependency update, or choosing a local file is appropriate. Disk-full, permission, unavailable codec, and malformed-media errors preserve completed outputs and allow retry. Cancel stops active processing and marks incomplete outputs as incomplete. Closing the app while work runs offers continue or cancel-and-close.

## 10. Delivery phases

1. Foundation: packaged app, local import/probe, one audio conversion, reliable queue and outputs.
2. Downloads: YouTube extraction, quality choices, stream retention, dependency diagnostics.
3. Editing: trim/split parser, preview, metadata/artwork, bulk operations.
4. Release: clean-machine installer tests, interruption recovery, accessibility and failure testing.

Planning estimate for one developer: roughly 1–2 weeks for a narrow working prototype and 4–8 weeks for a distributable P0 at part-time pace. These are scope estimates, not commitments; packaging and download-source changes are the main uncertainty.

# Part II — Technical Requirements Document

## 1. Architecture decision

Use Tauri 2 + React + TypeScript for the desktop shell and UI, with a Rust application backend for validation, orchestration, metadata, and SQLite persistence. Bundle yt-dlp, FFmpeg, and ffprobe as external executables. There is no custom Python worker, separate backend service, or local HTTP server. Users do not install Python; the bundled yt-dlp executable contains its own runtime where needed.

Tauri supports packaging external executables as sidecars; build each required binary for the target architecture [1]. Use a packaged JavaScript runtime and the extractor components required by the pinned yt-dlp release; validate extraction in the actual distributed build. Dependency versions are release-pinned and recorded in diagnostics.

| Component | Responsibility |
|---|---|
| React UI | Source selection, previews, editable plans, job views |
| Tauri commands | Typed frontend/backend IPC, native dialogs, scoped file access |
| Rust application core | Validation, probing, job planning, subprocess supervision, tags, persistence |
| yt-dlp adapter | Source inspection/download and normalized progress |
| FFmpeg/ffprobe adapter | Media inspection, extraction, conversion, trimming, validation |
| lofty adapter | Rust format-specific music metadata and embedded artwork |
| rusqlite / SQLite | Projects, immutable job plans, artifacts, checkpoints, history |

React cannot invoke arbitrary executables or arbitrary shell commands. The Rust backend exposes application-specific operations only. A second instance opens the existing application rather than starting another scheduler against the same database.

## 2. IPC contract

React invokes typed Tauri commands implemented in Rust. Request/response DTOs use serde serialization, explicit validation, and stable error codes; TypeScript types are generated from the Rust contracts or checked against shared fixtures in CI. Long-running commands return a job ID immediately. Rust sends progress and result events through Tauri events/channels; each job event contains a job ID, attempt ID, and monotonic sequence number. Subscribe before enqueueing and reconcile through list_jobs after reconnect or UI reload. No custom worker-process IPC protocol is required.

Commands: `inspect_url`, `inspect_file`, `parse_segments`, `save_project`, `plan_export`, `enqueue_job`, `cancel_job`, `retry_job`, `list_jobs`, `get_artifacts`, `get_diagnostics`.

Events: `source_ready`, `job_state_changed`, `stage_progress`, `artifact_ready`, `job_error`.

`plan_export` returns selected streams, transformations, compatibility warnings, destinations, expected artifact count, and a plan hash. `enqueue_job` consumes that stored plan. Changes to source identity or export settings invalidate it. Secrets and signed download URLs are never persisted in plans or logs.

### Rust modules and execution model

Keep business logic independent of Tauri so core tests run without a webview. Suggested modules: `domain`, `commands`, `jobs`, `downloads`, `media`, `tags`, `storage`, `paths`, and `diagnostics`.

Use Tokio tasks for scheduling and asynchronous subprocess supervision; use `tokio::process::Command` with argument arrays and shell execution disabled. Run blocking tag/image operations on bounded blocking tasks. Use cancellation tokens plus a process supervisor; dropping a future alone is not cancellation. On Windows, assign subprocesses to a Job Object so cancellation and shutdown terminate descendants, including FFmpeg started by yt-dlp.

Use rusqlite with bundled SQLite [4]. A dedicated database thread owns its connection and receives requests over channels; never block async/UI threads or share a connection concurrently without synchronization. Use transactions for state/artifact transitions and versioned schema migrations with backups. Use `lofty` for tags, `serde`/`serde_json` for DTOs and tool output, and `tracing` for sanitized diagnostics. Pin reviewed crate versions in Cargo.lock.

## 3. Data model

| Entity | Essential fields |
|---|---|
| Project | ID, name, created/updated time, schema version |
| Source | ID, project ID, URL or local path, provider ID, duration, stream descriptors, probe fingerprint |
| Segment | ID, source ID, start/end in integer microseconds, title, order, tag overrides |
| TagSet | Shared/per-segment values; explicit unchanged/set/remove semantics; artwork reference |
| Artwork | ID, local managed path, MIME type, dimensions, content hash |
| ExportPlan | ID/hash, source fingerprint, selected streams, codec/container settings, segments, paths, dependency versions |
| Job | ID, plan ID, state, stage, progress, attempt, timestamps, sanitized error |
| Artifact | ID, job ID, segment ID, staged/final path, validation result, size |

Store raw provider metadata separately from normalized application fields. Fingerprint local inputs with path, size and modification time; use hashes where reuse correctness requires them. Revalidate before starting an export.

## 4. Job execution and recovery

State transitions: queued → running → succeeded/failed; running → cancelling → cancelled. An application crash turns a previously running job into interrupted at next launch. An interrupted job requires retry; never display it as successful.

Stages: inspect, acquire, transform, tag, validate, finalize. Progress is stage-specific. Download totals can be unknown; show indeterminate progress rather than inventing a percentage. Start with at most two acquisition tasks and one transformation task concurrently; shared source acquisition occurs once per plan group.

Every attempt uses a unique working directory. Keep source downloads for reuse until the configured cache limit is exceeded; never evict active-job inputs. Reuse completed downloads only after validating identity and completeness. Resume partial network downloads only where the underlying extractor supports it. Completed tracks survive a later track failure, and retries skip validated artifacts from the same immutable plan.

Write outputs to staging files on the destination filesystem, preserving a recognizable container suffix. Probe and validate, then atomically finalize without overwriting existing files. Allocate collision-safe names at finalization as well as planning. In-place tag editing uses a staged copy and backup. Cancelling terminates the full subprocess tree, waits for exit, and removes incomplete staged outputs while retaining reusable source downloads.

## 5. Download integration

Call the bundled yt-dlp executable directly from Rust behind a versioned adapter. Inspect with its documented JSON output and normalize fields into application DTOs. Consume explicitly configured progress templates and final-path output, keeping those machine-readable records separate from diagnostic stderr [2]. Drain both streams concurrently, bound record sizes, and tolerate unavailable fields. Disable user/global yt-dlp configuration for reproducible execution, and pass the bundled FFmpeg/runtime locations explicitly. Finalization must use reported/probed paths, not guesses from the source title.

Inspect without downloading, list available streams, and select from actual availability. Do not hardcode YouTube format IDs such as 251: they are source-specific. Store selected format IDs for the immediate plan, but refresh expiring media URLs at acquisition time. Best audio uses the extractor's current quality ranking; cross-codec bitrates are not a universal quality score.

Default video choice may require separate video/audio streams and a merge. If those streams cannot be copied into the requested container, offer a compatible container or an explicit transcode. Single-item inspection disables incidental playlist expansion. P0 does not import browser cookies or handle account authentication; identify restricted sources clearly.

## 6. Media processing

Probe with ffprobe JSON. Normalize duration, streams, codecs, sample rates, channels, dimensions, frame rates, and tags. Reject missing audio for audio-only jobs and missing video for video jobs. Treat unreliable duration as unknown until validated.

Construct FFmpeg argument arrays with shell execution disabled. Use `-progress pipe:1` for structured progress and drain stderr concurrently. Select streams explicitly. Output codec/container combinations are generated from a supported capability matrix, not arbitrary user flags.

| Preset | Processing policy |
|---|---|
| Original audio | Stream copy into a suitable source-compatible container |
| MP3 | libmp3lame, selected bitrate; 320 kbps high-quality preset |
| M4A | AAC in MP4 audio container; copy existing compatible AAC when no audio transform is required |
| FLAC / WAV | Decode and encode losslessly or to PCM; explain source limitations |
| Original video | Copy selected compatible audio/video streams; choose suitable container |
| Compatible MP4 | H.264 + AAC; preserve compatible existing streams when possible |

For splitting, encode each region directly from the original. Accurate mode decodes/trims with normalized timestamps; fast copy mode accepts packet/keyframe granularity. Validate actual output durations with codec-aware tolerances. Exclude inherited chapters from split tracks unless deliberately regenerated. P1 joins copy only truly compatible streams; otherwise normalize and re-encode with an explicit plan.

Use a lightweight preview proxy when the webview cannot decode the original codec; keep the original as the export source. Waveform peaks, if implemented, are cached separately and do not load entire recordings into frontend memory.

## 7. Metadata and artwork

Use a normalized tag model with format-specific adapters: ID3 for MP3, MP4 atoms for M4A, and Vorbis-style comments/pictures for FLAC. Use lofty behind format-specific adapters; its documented format matrix covers MP3, MP4, FLAC, and other audio formats [3]. Validate picture and field support against the pinned version before promising a capability. Use ID3v2.3 for the default MP3 compatibility preset, with documented conversion for fields that differ from v2.4.

Embed JPEG/PNG front-cover artwork; decode and validate images, apply orientation, and cap accepted images at 20 MB and 40 megapixels. Preserve original artwork separately and generate an export-sized image up to 1500×1500 without upscaling. Do not force a square crop without user choice. Support remove/replace distinctly. Generic MP4 video tagging is limited to fields supported by its adapter; music artwork behavior is guaranteed only for tested audio formats.

Tag edits must not re-encode audio. Apply explicit changes to staged copies and preserve unrelated supported tags. New split exports receive resolved shared/per-segment metadata rather than stale chapter or source titles. Filename sanitization never changes the stored display title.

## 8. Packaging, updates, and boundaries

Ship a Windows x64 installer containing the Tauri/Rust application, yt-dlp, FFmpeg/ffprobe, required extractor runtime, and licenses. No separately packaged Python application backend is shipped. Runtime resources resolve by application paths, not PATH or current working directory. Test filenames with Unicode, quotes, long names, reserved Windows device names, and network/removable destinations.

Initially update dependencies through versioned app releases, preserving a last-known-working bundle. Provide a diagnostics view with version information and “check for updates.” Do not let yt-dlp self-update a signed bundled executable in place. An independent extractor updater is P1 and requires authenticated version manifests, integrity verification, rollback, and compatibility checks.

Disable remote web content in the privileged UI. Permit only required Tauri capabilities. Treat URLs, metadata, artwork, and media as untrusted inputs. Use argument arrays, controlled temporary paths, output-directory containment checks, bounded metadata/image sizes, and sanitized logs. The app processes media locally; only source downloads and explicit update checks require network access. No accounts or telemetry in P0.

Before distribution, inventory exact dependency/build licenses and satisfy their notices/source obligations; FFmpeg licensing depends on enabled build components. This is a release engineering requirement, not an assumption that every binary build is interchangeable.

## 9. Verification and release gates

| Area | Required checks |
|---|---|
| Parser | M:SS/H:MM:SS, pasted links, duplicates, decreasing starts, bounds, invalid lines, unused intro |
| Planner | Container/codec compatibility, copy/transcode choice, collisions, immutable retry plans |
| Media fixtures | AAC/Opus/MP3/FLAC, audio/video, variable frame rate, corrupt and missing streams |
| Quality | Stream-copy codec preservation; single encode from original; trim tolerances per output codec |
| Tags/artwork | Read-back values, cover MIME/type, bulk mixed fields, remove semantics, audio unchanged after tag edits |
| Lifecycle | Cancel during each stage, kill/restart recovery, partial batch retry, disk-full, permission failures |
| Packaging | Clean Windows VM, no dependency PATH, offline local operations, paths containing spaces/Unicode |
| Downloads | Mocked extractor failures plus separate best-effort live smoke tests; never depend on one video for core CI |

P0 ships when all listed product workflows pass on a clean supported machine, failed/cancelled jobs cannot corrupt sources, and exported metadata/artwork is verified with independent read-back. Test playback in representative local media players; Spotify Local Files is a useful compatibility smoke test, not a guarantee across every Spotify client.

## 10. Remaining decisions

Validate whether Windows x64 alone satisfies the first release audience; choose the product name; decide whether playlist support belongs in the first public release; and confirm installer signing/distribution. These decisions do not block a local-file prototype. Dependency versions are selected and locked during implementation, after packaging a full extraction/export smoke test.

## Technical references

1. Tauri 2, Embedding External Binaries: https://v2.tauri.app/develop/sidecar/
2. yt-dlp official README, embedding, formats and progress: https://github.com/yt-dlp/yt-dlp/blob/master/README.md
3. lofty Rust API and supported metadata formats: https://docs.rs/lofty/latest/lofty/
4. rusqlite Rust API and SQLite integration: https://docs.rs/rusqlite/latest/rusqlite/

References ground dependency capabilities. Product scope, architecture choices, targets, and estimates are proposed requirements rather than claims from those sources.
