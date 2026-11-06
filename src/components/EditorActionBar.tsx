import { ArrowUpRight, ChevronRight, Save } from 'lucide-react';
import { desktop } from '../desktop';
import type { Tab } from '../types';

interface EditorActionBarProps {
  tab: Tab;
  setTab: (tab: Tab) => void;
  busy: string;
  totalRegions: number;
  review: () => Promise<void>;
}

export function EditorActionBar({ tab, setTab, busy, totalRegions, review }: EditorActionBarProps) {
  return (
    <div className="editor-actionbar">
      <span>
        <Save size={15} />
        {desktop ? 'Workspace saved locally' : 'Desktop preview'}
      </span>
      <div className="button-row">
        {tab !== 'export' ? (
          <button className="button primary" onClick={() => setTab('export')}>
            Export settings <ChevronRight size={16} />
          </button>
        ) : (
          <button
            className="button primary"
            disabled={!!busy || !totalRegions}
            onClick={() => void review()}
          >
            Review {totalRegions} {totalRegions === 1 ? 'output' : 'outputs'}{' '}
            <ArrowUpRight size={17} />
          </button>
        )}
      </div>
    </div>
  );
}
