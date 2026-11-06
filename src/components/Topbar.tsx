import { ChevronRight, Link, Plus } from 'lucide-react';
import type { Page } from '../types';

interface TopbarProps {
  page: Page;
  setUrlOpen: (open: boolean) => void;
  pickFiles: () => Promise<void>;
}

export function Topbar({ page, setUrlOpen, pickFiles }: TopbarProps) {
  return (
    <header className="topbar">
      <div className="breadcrumb">
        Workspace <ChevronRight size={14} />
        <strong>{page === 'jobs' ? 'Jobs' : 'Editor'}</strong>
      </div>
      <div className="topbar-right">
        <button className="button secondary compact" onClick={() => setUrlOpen(true)}>
          <Link size={15} /> Paste URL
        </button>
        <button className="button primary compact" onClick={() => void pickFiles()}>
          <Plus size={17} /> Add files
        </button>
      </div>
    </header>
  );
}
