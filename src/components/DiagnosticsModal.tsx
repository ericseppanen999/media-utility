import { RefreshCw } from 'lucide-react';
import { desktop } from '../desktop';
import type { Diagnostics } from '../types';
import { Modal } from './Modal';

interface DiagnosticsModalProps {
  diagnostics: Diagnostics | null;
  setDiagnosticsOpen: (open: boolean) => void;
  refreshDiagnostics: () => Promise<void>;
  error: string;
}

export function DiagnosticsModal({
  diagnostics,
  setDiagnosticsOpen,
  refreshDiagnostics,
  error,
}: DiagnosticsModalProps) {
  return (
    <Modal
      error={error}
      title="Diagnostics"
      subtitle="Installed tool versions and paths."
      close={() => setDiagnosticsOpen(false)}
    >
      {diagnostics ? (
        <>
          <div className="diagnostic-list">
            {Object.entries(diagnostics.tools).map(([name, version]) => (
              <div key={name}>
                <strong>{name}</strong>
                <span className={version.includes('Missing') ? 'warning' : ''}>{version}</span>
              </div>
            ))}
          </div>
          <p className="hint">
            {diagnostics.database}
            <br />
            {diagnostics.tool_directory}
          </p>
          <p className="hint">
            Dependencies are updated with application releases. An in-app update channel is not
            configured.
          </p>
          <button className="button secondary" onClick={() => void refreshDiagnostics()}>
            <RefreshCw size={15} /> Check installed versions
          </button>
        </>
      ) : (
        <p>
          {desktop
            ? 'Checking bundled dependencies…'
            : 'Diagnostics are available in the desktop app.'}
        </p>
      )}
    </Modal>
  );
}
