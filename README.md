# Media Utility

A local Windows desktop media editor built with Tauri 2, React, TypeScript, and Rust.

## Run

This workspace has its npm dependencies and checksum-verified Windows media tools prepared.

~~~powershell
npm run desktop
~~~

The first native build takes several minutes. Subsequent launches reuse compiled dependencies. The command opens the desktop app and keeps frontend hot reload active.

For a fresh checkout, install Node, Rust, Microsoft C++ build tools, and WebView2, then:

~~~powershell
npm ci
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-tools.ps1
npm run desktop
~~~

The preparation script uses the versions and SHA-256 hashes in scripts/dependencies.lock.json. Users of a packaged build do not need Node, Rust, Python, FFmpeg, or yt-dlp on PATH.

## Current workflows

- Import multiple local files through the picker or native drag/drop.
- Inspect public YouTube links and choose an available download format.
- Preview local media; create a compatible preview copy when necessary.
- Trim or split with numeric bounds, pasted timestamps, or source chapters.
- Edit tags and JPEG/PNG cover art, including bulk unchanged/set/remove operations.
- Review MP3, M4A, FLAC, WAV, original-audio, original-video, and compatible MP4 plans.
- Export staged, validated copies with collision-safe filenames.
- Cancel and retry jobs; persist job history and completed artifacts in SQLite.

Choose files, open **Segments** to trim, then open **Export**, choose a destination, and review the plan before starting. Ctrl-click sources or use **Select all sources** for bulk tags.

A browser-only preview is available with npm run dev. Native media operations require the desktop app; the browser preview does not fabricate processing results.

## Verification

~~~powershell
npm run build
npm test
npm exec playwright test
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --test media_pipeline -- --include-ignored
~~~

The last command requires prepared tools and performs real conversions. Browser tests use Microsoft Edge and a native IPC fixture; they do not substitute for native media tests.

## Packaging

~~~powershell
npm run desktop:build
~~~

Tauri is configured for a Windows NSIS installer and bundles the binaries directory. Distribution is not yet release-approved. Signing, full dependency/license obligations, clean-machine testing, and the specification's remaining acceptance gates still need completion.

## Implementation boundaries

This is an initial implementation, not a declaration that every P0 release gate in media-utility-prd-trd.md has passed.

- No playlist expansion, account authentication, cookies, DRM handling, or cloud services.
- Original-video output uses Matroska; original online audio uses Matroska audio until acquisition. Unsupported music tagging is rejected rather than silently discarded.
- Download progress is indeterminate; transformation progress uses FFmpeg records.
- A cached complete download may be reused by the same immutable plan after hash/probe validation. Automatic cache eviction and resumable acquisition across attempts are not implemented.
- Preview proxy creation is separate from the export queue.
- URL thumbnails and a linked-timestamp shortcut remain unimplemented; numeric trim is available.
- In-place overwrite/backup editing, automatic updates, and P1 features are not implemented.
- Format coverage, metadata interoperability, filesystem interruption windows, and failure behavior need the wider fixture/clean-machine release matrix in the specification.

Local application state lives in the platform app-data directory under the identifier local.mediautility.desktop. Tool locations are resolved from the application bundle, never PATH.
