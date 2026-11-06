import { Scissors } from 'lucide-react';
import { Modal } from './Modal';

interface TimestampsModalProps {
  timestamps: string;
  setTimestamps: (text: string) => void;
  setPasteOpen: (open: boolean) => void;
  parseSegments: () => Promise<void>;
  busy: string;
  error: string;
}

export function TimestampsModal({
  timestamps,
  setTimestamps,
  setPasteOpen,
  parseSegments,
  busy,
  error,
}: TimestampsModalProps) {
  return (
    <Modal
      error={error}
      title="Paste timestamps"
      subtitle="One entry per line. The next start becomes the previous end."
      close={() => setPasteOpen(false)}
    >
      <textarea
        autoFocus
        rows={8}
        value={timestamps}
        onChange={(e) => setTimestamps(e.target.value)}
        placeholder={'0:00 Track 1\n3:42 Track 2\n8:15 Track 3'}
      />
      <p className="hint">Accepts M:SS, H:MM:SS, and copied timestamp links.</p>
      <div className="modal-actions">
        <button className="button secondary" onClick={() => setPasteOpen(false)}>
          Cancel
        </button>
        <button
          className="button primary"
          disabled={!timestamps || !!busy}
          onClick={() => void parseSegments()}
        >
          <Scissors size={16} /> Create regions
        </button>
      </div>
    </Modal>
  );
}
