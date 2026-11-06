import { open } from '@tauri-apps/plugin-dialog';
import { Check, FolderOpen, Info } from 'lucide-react';
import type { Preset, Source, Work } from '../types';

const presets: { id: Preset; title: string; detail: string }[] = [
  { id: 'mp3', title: 'MP3', detail: 'Broad compatibility' },
  { id: 'm4a', title: 'M4A / AAC', detail: 'Efficient audio' },
  { id: 'flac', title: 'FLAC', detail: 'Lossless audio' },
  { id: 'wav', title: 'WAV', detail: 'Uncompressed audio' },
  { id: 'original_audio', title: 'Original audio', detail: 'Copy audio stream' },
  { id: 'mp4', title: 'MP4', detail: 'H.264 + AAC video' },
  { id: 'original_video', title: 'Original video', detail: 'Copy to Matroska' },
];

interface ExportTabProps {
  source: Source;
  chosen: Source[];
  preset: Preset;
  setPreset: (preset: Preset) => void;
  bitrate: number;
  setBitrate: (bitrate: number) => void;
  formatId: string;
  setFormatId: (id: string) => void;
  destination: string;
  setDestination: (path: string) => void;
  totalRegions: number;
  work: Work;
}

export function ExportTab({
  source,
  chosen,
  preset,
  setPreset,
  bitrate,
  setBitrate,
  formatId,
  setFormatId,
  destination,
  setDestination,
  totalRegions,
  work,
}: ExportTabProps) {
  return (
    <>
      <div className="section-toolbar">
        <div>
          <h2>Export settings</h2>
          <p>Choose a format and output folder.</p>
        </div>
      </div>
      <div className="export-layout">
        <div>
          <label className="group-label">OUTPUT FORMAT</label>
          <div className="preset-grid">
            {presets.map((p) => (
              <button
                key={p.id}
                className={'preset ' + (preset === p.id ? 'chosen' : '')}
                onClick={() => setPreset(p.id)}
              >
                <span>
                  <strong>{p.title}</strong>
                  <small>{p.detail}</small>
                </span>
                <span className="radio-dot">{preset === p.id && <Check size={11} />}</span>
              </button>
            ))}
          </div>
          {['mp3', 'm4a', 'mp4'].includes(preset) && (
            <div className="field bitrate-field">
              <label htmlFor="bitrate">Audio bitrate</label>
              <select
                id="bitrate"
                value={bitrate}
                onChange={(e) => setBitrate(Number(e.target.value))}
              >
                {[96, 128, 160, 192, 256, 320].map((b) => (
                  <option key={b} value={b}>
                    {b} kbps{b === 320 ? ' · High quality' : ''}
                  </option>
                ))}
              </select>
              <p className="hint">
                A higher bitrate cannot restore detail missing from the source.
              </p>
            </div>
          )}
          {source.url && chosen.length === 1 && (
            <div className="field">
              <label htmlFor="download-format">Download format</label>
              <select
                id="download-format"
                value={formatId}
                onChange={(e) => setFormatId(e.target.value)}
              >
                <option value="">Best available · automatic</option>
                {source.formats.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.label}
                  </option>
                ))}
              </select>
            </div>
          )}
        </div>
        <div className="panel output-panel">
          <div className="panel-heading">
            <FolderOpen size={17} /> Export destination
          </div>
          <p className="folder-path">{destination || 'No folder selected'}</p>
          <button
            className="button secondary full"
            onClick={() =>
              void work('Choosing destination', async () => {
                const path = await open({ directory: true, multiple: false });
                if (typeof path === 'string') setDestination(path);
              })
            }
          >
            <FolderOpen size={16} /> Choose folder
          </button>
          <div className="output-facts">
            <span>
              Selected sources<strong>{chosen.length}</strong>
            </span>
            <span>
              Expected files<strong>{totalRegions}</strong>
            </span>
          </div>
          <div className="inline-note">
            <Info size={16} />
            <p>Names use track numbers and titles. Existing files are never overwritten.</p>
          </div>
        </div>
      </div>
    </>
  );
}
