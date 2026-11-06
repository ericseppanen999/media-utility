export type Edit =
  { action: 'unchanged' } | { action: 'remove' } | { action: 'set'; value: string };

export type Preset = 'original_audio' | 'mp3' | 'm4a' | 'flac' | 'wav' | 'original_video' | 'mp4';

export interface Segment {
  id: string;
  start_us: number;
  end_us: number;
  title: string;
}

export interface Stream {
  index: number;
  kind: string;
  codec: string;
  sample_rate: string | null;
  channels: number | null;
  width: number | null;
  height: number | null;
}

export interface Source {
  id: string;
  title: string;
  path: string | null;
  url: string | null;
  duration_us: number | null;
  fingerprint: string;
  streams: Stream[];
  chapters: Segment[];
  tags: Record<string, string>;
  formats: { id: string; label: string; audio: boolean; video: boolean }[];
  thumbnail: string | null;
}

export interface ExportRequest {
  source_id: string;
  destination: string;
  preset: Preset;
  bitrate: number;
  segments: Segment[];
  independent_clips: boolean;
  tags: Record<string, Edit>;
  artwork: Edit;
  format_id: string | null;
}

export interface Plan {
  id: string;
  hash: string;
  source: Source;
  request: ExportRequest;
  outputs: { segment: Segment; path: string }[];
  extension: string;
  copy: boolean;
  explanation: string;
  warnings: string[];
  dependencies: Record<string, string>;
}

export interface Artifact {
  id: string;
  segment_id: string;
  path: string;
  size: number;
}

export interface Job {
  id: string;
  plan_id: string;
  title: string;
  state: string;
  stage: string;
  progress: number | null;
  attempt: number;
  sequence: number;
  error: { code: string; message: string } | null;
  artifacts: Artifact[];
  created_at: number;
  destination: string;
}

export interface Diagnostics {
  app_version: string;
  platform: string;
  tools: Record<string, string>;
  tool_directory: string;
  database: string;
}

export interface EditorState {
  segments: Segment[];
  tags: Record<string, Edit>;
  artwork: Edit;
}

export type Page = 'editor' | 'jobs';

export type Tab = 'preview' | 'segments' | 'tags' | 'export';

/** Runs an async action under a busy label, surfacing any failure as the app error. */
export type Work = (label: string, fn: () => Promise<void>) => Promise<void>;
