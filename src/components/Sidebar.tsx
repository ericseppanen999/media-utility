import { AudioLines, Download, FileAudio, Film, Layers, Plus, Settings2 } from 'lucide-react';
import { time } from '../lib';
import type { Job, Page, Source } from '../types';

interface SidebarProps {
  page: Page;
  setPage: (page: Page) => void;
  activeJobs: Job[];
  sources: Source[];
  selected: string[];
  setSelected: React.Dispatch<React.SetStateAction<string[]>>;
  pickFiles: () => Promise<void>;
  refreshDiagnostics: () => Promise<void>;
}

export function Sidebar({
  page,
  setPage,
  activeJobs,
  sources,
  selected,
  setSelected,
  pickFiles,
  refreshDiagnostics,
}: SidebarProps) {
  return (
    <aside className="sidebar">
      <a
        className="brand"
        href="#"
        onClick={(e) => {
          e.preventDefault();
          setPage('editor');
        }}
      >
        <span className="brand-symbol">
          <AudioLines size={23} />
        </span>
        <span>Media Utility</span>
      </a>
      <div className="workspace-label">WORKSPACE</div>
      <button
        className={'nav-item ' + (page === 'editor' ? 'selected' : '')}
        onClick={() => setPage('editor')}
      >
        <Layers size={18} /> Editor <span className="nav-key">1</span>
      </button>
      <button
        className={'nav-item ' + (page === 'jobs' ? 'selected' : '')}
        onClick={() => setPage('jobs')}
      >
        <Download size={18} /> Jobs{' '}
        {activeJobs.length > 0 ? (
          <span className="count">{activeJobs.length}</span>
        ) : (
          <span className="nav-key">2</span>
        )}
      </button>
      <div className="source-heading">
        <span>
          SOURCES <b>{sources.length}</b>
        </span>
        <button className="icon-button" aria-label="Import files" onClick={() => void pickFiles()}>
          <Plus size={16} />
        </button>
      </div>
      <div className="source-list">
        {!sources.length ? (
          <div className="sidebar-empty">
            <FileAudio size={24} />
            <p>No sources</p>
            <span>Import a file or paste a link.</span>
          </div>
        ) : (
          sources.map((s) => (
            <button
              key={s.id}
              className={'source-item ' + (selected.includes(s.id) ? 'selected' : '')}
              onClick={(event) => {
                setSelected((old) =>
                  event.ctrlKey || event.metaKey
                    ? old.includes(s.id)
                      ? old.filter((id) => id !== s.id)
                      : [...old, s.id]
                    : [s.id],
                );
                setPage('editor');
              }}
              title={s.path ?? s.url ?? s.title}
            >
              <span className="source-icon">
                {s.streams.some((s) => s.kind === 'video') ? (
                  <Film size={17} />
                ) : (
                  <FileAudio size={17} />
                )}
              </span>
              <span className="source-text">
                <strong>{s.title}</strong>
                <small>
                  {s.url ? 'YouTube' : 'Local file'} <i /> {time(s.duration_us)}
                </small>
              </span>
              {selected.includes(s.id) && <span className="selection-dot" />}
            </button>
          ))
        )}
      </div>
      {sources.length > 1 && (
        <button
          className="text-button select-all"
          onClick={() => setSelected(sources.map((s) => s.id))}
        >
          Select all sources
        </button>
      )}
      <div className="sidebar-bottom">
        <button className="nav-item" onClick={() => void refreshDiagnostics()}>
          <Settings2 size={17} /> Diagnostics <span className="version">v0.1</span>
        </button>
      </div>
    </aside>
  );
}
