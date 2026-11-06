import { convertFileSrc } from '@tauri-apps/api/core';
import { ImagePlus, X } from 'lucide-react';
import { unchanged } from '../editor';
import type { Edit, EditorState, Source } from '../types';

const fields = [
  ['title', 'Title'],
  ['artist', 'Artist'],
  ['album_artist', 'Album artist'],
  ['album', 'Album'],
  ['track', 'Track'],
  ['track_total', 'Total tracks'],
  ['disc', 'Disc'],
  ['disc_total', 'Total discs'],
  ['year', 'Year'],
  ['genre', 'Genre'],
  ['comment', 'Comment'],
];

interface TagsTabProps {
  chosen: Source[];
  editors: Record<string, EditorState>;
  artwork: Edit | undefined;
  tagValue: (key: string) => { value: string; mixed: boolean };
  updateTag: (key: string, edit: Edit) => void;
  setArtwork: (edit: Edit) => void;
  pickArtwork: () => Promise<void>;
}

export function TagsTab({
  chosen,
  editors,
  artwork,
  tagValue,
  updateTag,
  setArtwork,
  pickArtwork,
}: TagsTabProps) {
  return (
    <>
      <div className="section-toolbar">
        <div>
          <h2>Tags and artwork</h2>
          <p>
            {chosen.length > 1
              ? 'Changes apply to all ' +
                chosen.length +
                ' selected sources. Mixed fields are unchanged unless edited.'
              : 'Edit file metadata. Split tracks use their region titles and numbers.'}
          </p>
        </div>
        <span className="pill">{chosen.length > 1 ? 'Bulk editing' : 'Metadata'}</span>
      </div>
      <div className="tag-layout">
        <div className="tag-fields">
          {fields.map(([key, label]) => {
            const value = tagValue(key);
            const modified = chosen.some(
              (s) =>
                editors[s.id]?.tags[key]?.action && editors[s.id].tags[key].action !== 'unchanged',
            );
            return (
              <div key={key} className={'field ' + (key === 'comment' ? 'wide' : '')}>
                <label htmlFor={'tag-' + key}>
                  {label}
                  {modified && <span className="edited">Edited</span>}
                </label>
                <div className="field-with-action">
                  <input
                    id={'tag-' + key}
                    value={value.value}
                    placeholder={value.mixed ? 'Mixed — unchanged' : 'Not set'}
                    onChange={(e) => updateTag(key, { action: 'set', value: e.target.value })}
                  />
                  <button
                    className="icon-button"
                    aria-label={'Remove ' + label}
                    title={'Remove ' + label}
                    onClick={() => updateTag(key, { action: 'remove' })}
                  >
                    <X size={14} />
                  </button>
                </div>
                {modified && (
                  <button className="reset-field" onClick={() => updateTag(key, unchanged)}>
                    Keep original
                  </button>
                )}
              </div>
            );
          })}
        </div>
        <div className="artwork-column">
          <label>Cover artwork</label>
          <button className="artwork-box" onClick={() => void pickArtwork()}>
            {artwork?.action === 'set' ? (
              <img src={convertFileSrc(artwork.value)} alt="Selected cover artwork" />
            ) : (
              <>
                <ImagePlus size={32} />
                <strong>
                  {artwork?.action === 'remove' ? 'Artwork removed' : 'Add cover art'}
                </strong>
                <span>JPEG or PNG</span>
              </>
            )}
          </button>
          <p className="hint">
            Up to 20 MB. Kept in proportion,
            <br />
            with a maximum edge of 1500 px.
          </p>
          <div className="button-row">
            <button className="text-button" onClick={() => setArtwork({ action: 'remove' })}>
              Remove
            </button>
            <button className="text-button muted" onClick={() => setArtwork(unchanged)}>
              Keep original
            </button>
          </div>
        </div>
      </div>
    </>
  );
}
