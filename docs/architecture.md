# Architecture

The React UI only invokes application-specific Tauri commands. No general shell or filesystem plugin is exposed to the UI.

The Rust core is independent of the webview:

- domain: serializable source, editor, plan, job, artifact, and error types.
- segments: timestamp/link parsing and sequential versus overlapping region validation.
- paths: Windows filename sanitization, fingerprints, hashes, and non-replacing finalization.
- storage: one database thread, SQLite WAL, serialized requests, transactional record updates, and startup interruption recovery.
- process: argument-array child invocation, bounded stdout/stderr readers, cancellation tokens, and Windows Job Objects.
- media and downloads: FFmpeg/ffprobe and yt-dlp adapters, with bundled Deno.
- planner: immutable export snapshots, supported codec/container choices, filenames, explanations, and warnings.
- tags: Lofty metadata writing/read-back and bounded artwork preparation.
- jobs: two acquisition permits, one transformation permit, staged exports, artifact checkpoints, retries, and events.
- commands: Tauri integration, native preview scope, diagnostics, and reveal-output operations.

Each job event includes job ID, attempt, and increasing sequence. The UI subscribes before loading persisted jobs and ignores stale events. A source's size, modification time, and canonical path are rechecked before processing. Completed artifact hashes are checked before retry skips them.

The database currently uses a version-1 records schema with typed serialized documents. A future schema change must add a backup and migration rather than simply changing the schema version.

The filesystem and SQLite cannot participate in one atomic transaction. Final files are never replaced, but a crash between finalization and the artifact checkpoint can leave a valid output that is not recorded. Full reconciliation of that window remains a release-hardening task.
