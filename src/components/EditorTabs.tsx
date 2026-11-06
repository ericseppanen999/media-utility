import { Play, Scissors, SlidersHorizontal, Tags } from 'lucide-react';
import type { EditorState, Tab } from '../types';

interface EditorTabsProps {
  tab: Tab;
  setTab: (tab: Tab) => void;
  editor: EditorState | null;
}

export function EditorTabs({ tab, setTab, editor }: EditorTabsProps) {
  return (
    <div className="editor-tabs" role="tablist">
      {(
        [
          { id: 'preview', label: 'Preview', icon: Play },
          { id: 'segments', label: 'Segments', icon: Scissors },
          { id: 'tags', label: 'Tags & artwork', icon: Tags },
          { id: 'export', label: 'Export', icon: SlidersHorizontal },
        ] as const
      ).map((t) => (
        <button
          role="tab"
          aria-selected={tab === t.id}
          key={t.id}
          className={tab === t.id ? 'active' : ''}
          onClick={() => setTab(t.id)}
        >
          <t.icon size={16} />
          {t.label}
          {t.id === 'segments' && <span>{editor?.segments.length}</span>}
        </button>
      ))}
    </div>
  );
}
