import { Info, Plus, Trash2 } from 'lucide-react';
import { time } from '../lib';
import type { EditorState, Segment, Source } from '../types';
import { TimeInput } from './TimeInput';

interface SegmentsTabProps {
  source: Source;
  editor: EditorState;
  patchEditor: (patch: Partial<EditorState>) => void;
  updateSegment: (id: string, patch: Partial<Segment>) => void;
  setPasteOpen: (open: boolean) => void;
  independent: boolean;
  setIndependent: (independent: boolean) => void;
}

export function SegmentsTab({
  source,
  editor,
  patchEditor,
  updateSegment,
  setPasteOpen,
  independent,
  setIndependent,
}: SegmentsTabProps) {
  return (
    <>
      <div className="section-toolbar">
        <div>
          <h2>Trim and split</h2>
          <p>Set start and end times for each output file.</p>
        </div>
        <div className="button-row">
          <button
            className="button secondary compact"
            disabled={!source.chapters.length}
            onClick={() => patchEditor({ segments: source.chapters })}
          >
            Import chapters
          </button>
          <button className="button secondary compact" onClick={() => setPasteOpen(true)}>
            <Plus size={15} /> Paste timestamps
          </button>
        </div>
      </div>
      <div className="segments-table">
        <div className="segment-row table-header">
          <span>#</span>
          <span>TRACK TITLE</span>
          <span>START</span>
          <span>END</span>
          <span>DURATION</span>
          <span />
        </div>
        {editor.segments.map((segment, index) => (
          <div className="segment-row" key={segment.id}>
            <span className="track-number">{index + 1}</span>
            <input
              aria-label={'Region ' + (index + 1) + ' title'}
              value={segment.title}
              onChange={(e) => updateSegment(segment.id, { title: e.target.value })}
            />
            <TimeInput
              label={'Region ' + (index + 1) + ' start'}
              value={segment.start_us}
              onChange={(v) => updateSegment(segment.id, { start_us: v })}
            />
            <TimeInput
              label={'Region ' + (index + 1) + ' end'}
              value={segment.end_us}
              onChange={(v) => updateSegment(segment.id, { end_us: v })}
            />
            <span className="duration">{time(segment.end_us - segment.start_us)}</span>
            <button
              className="icon-button"
              aria-label={'Remove region ' + (index + 1)}
              onClick={() =>
                patchEditor({
                  segments: editor.segments.filter((s) => s.id !== segment.id),
                })
              }
            >
              <Trash2 size={15} />
            </button>
          </div>
        ))}
      </div>
      <div className="section-toolbar bottom">
        <button
          className="text-button"
          onClick={() =>
            patchEditor({
              segments: [
                ...editor.segments,
                {
                  id: crypto.randomUUID(),
                  start_us: editor.segments.at(-1)?.end_us ?? 0,
                  end_us: source.duration_us ?? 0,
                  title: 'New region',
                },
              ],
            })
          }
        >
          <Plus size={16} /> Add region
        </button>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={independent}
            onChange={(e) => setIndependent(e.target.checked)}
          />{' '}
          Allow overlapping independent clips
        </label>
      </div>
      {editor.segments[0]?.start_us > 0 && (
        <div className="inline-note">
          <Info size={16} />
          <p>{time(editor.segments[0].start_us)} of introduction is not included.</p>
        </div>
      )}
    </>
  );
}
