import { Info, Link } from 'lucide-react';
import { Modal } from './Modal';

interface UrlModalProps {
  url: string;
  setUrl: (url: string) => void;
  setUrlOpen: (open: boolean) => void;
  importUrl: () => Promise<void>;
  busy: string;
  error: string;
}

export function UrlModal({ url, setUrl, setUrlOpen, importUrl, busy, error }: UrlModalProps) {
  return (
    <Modal
      error={error}
      title="Import URL"
      subtitle="Inspect a public YouTube video before downloading."
      close={() => setUrlOpen(false)}
    >
      <label className="field">
        <span>Video URL</span>
        <input
          autoFocus
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://www.youtube.com/watch?v=…"
          onKeyDown={(e) => {
            if (e.key === 'Enter') void importUrl();
          }}
        />
      </label>
      <div className="inline-note">
        <Info size={17} />
        <p>
          Only the linked video is imported, even if the URL contains a playlist. Linked timestamps
          default to the full video; adjust the start in Segments.
        </p>
      </div>
      <div className="modal-actions">
        <button className="button secondary" onClick={() => setUrlOpen(false)}>
          Cancel
        </button>
        <button
          className="button primary"
          disabled={!url || !!busy}
          onClick={() => void importUrl()}
        >
          <Link size={16} /> Inspect source
        </button>
      </div>
    </Modal>
  );
}
