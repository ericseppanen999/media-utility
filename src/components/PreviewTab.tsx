import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { AudioLines, ChevronRight, Info, Link, Scissors } from 'lucide-react';
import { time } from '../lib';
import type { Source, Tab, Work } from '../types';

interface PreviewTabProps {
  source: Source;
  previewPath: string | null;
  setPreviewPath: (path: string | null) => void;
  previewError: boolean;
  setPreviewError: (failed: boolean) => void;
  busy: string;
  work: Work;
  setTab: (tab: Tab) => void;
}

export function PreviewTab({
  source,
  previewPath,
  setPreviewPath,
  previewError,
  setPreviewError,
  busy,
  work,
  setTab,
}: PreviewTabProps) {
  return (
    <div className="preview-grid">
      <div className="panel preview-panel">
        <div className="panel-heading">
          <span>Source preview</span>
          <span className="micro-label">{source.url ? 'ONLINE SOURCE' : 'ORIGINAL FILE'}</span>
        </div>
        <div
          className={
            'player-surface ' +
            (!source.streams.some((s) => s.kind === 'video') ? 'audio-only' : '')
          }
        >
          {source.path ? (
            <>
              <div className="audio-art">
                <AudioLines size={58} />
                <span>ORIGINAL SOURCE</span>
              </div>
              <video
                key={source.id}
                controls
                src={convertFileSrc(previewPath ?? source.path)}
                onError={() => setPreviewError(true)}
                preload="metadata"
              />
            </>
          ) : (
            <div className="online-preview">
              <Link size={38} />
              <h3>Ready to download</h3>
              <p>Downloaded when the job starts.</p>
            </div>
          )}
        </div>
        {previewError && (
          <p className="hint warning">
            This format needs a converted copy for playback.{' '}
            <button
              className="text-button"
              disabled={!!busy}
              onClick={() =>
                void work('Creating preview', async () => {
                  setPreviewPath(await invoke<string>('create_preview', { sourceId: source.id }));
                  setPreviewError(false);
                })
              }
            >
              Create preview
            </button>
          </p>
        )}
        <div className="source-footer">
          <span>{time(source.duration_us)} duration</span>
          <span>{source.streams.map((s) => s.codec).join(' / ')}</span>
        </div>
      </div>
      <div className="panel details-panel">
        <div className="panel-heading">Media details</div>
        <dl>
          <dt>Source</dt>
          <dd>{source.url ? 'YouTube' : 'Local file'}</dd>
          <dt>Duration</dt>
          <dd>{time(source.duration_us)}</dd>
          {source.streams.map((s, i) => (
            <div className="stream-detail" key={i}>
              <dt>{s.kind === 'audio' ? 'Audio stream' : 'Video stream'}</dt>
              <dd>
                {s.codec.toUpperCase()}
                {s.sample_rate && ' · ' + Number(s.sample_rate) / 1000 + ' kHz'}
                {s.channels && ' · ' + s.channels + ' ch'}
                {s.width && ' · ' + s.width + '×' + s.height}
              </dd>
            </div>
          ))}
        </dl>
        <div className="inline-note">
          <Info size={17} />
          <p>Export choices use the actual streams in this file, not its extension.</p>
        </div>
        <button className="button secondary full" onClick={() => setTab('segments')}>
          <Scissors size={16} /> Trim or split <ChevronRight size={16} />
        </button>
      </div>
    </div>
  );
}
