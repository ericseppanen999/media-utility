import { FolderOpen } from 'lucide-react';

interface EmptyWorkspaceProps {
  pickFiles: () => Promise<void>;
}

export function EmptyWorkspace({ pickFiles }: EmptyWorkspaceProps) {
  return (
    <div className="empty-workspace">
      <div className="import-zone" onDragOver={(e) => e.preventDefault()}>
        <h2>Import media</h2>
        <p>Drop audio or video here, or choose files.</p>
        <button className="button primary" onClick={() => void pickFiles()}>
          <FolderOpen size={18} /> Choose files
        </button>
        <span className="supported">MP3 · FLAC · WAV · MP4 · M4A · and more</span>
      </div>
    </div>
  );
}
