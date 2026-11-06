import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open, confirm } from '@tauri-apps/plugin-dialog';
import { AlertCircle, CheckCircle2, Info, LoaderCircle, X } from 'lucide-react';
import { DiagnosticsModal } from './components/DiagnosticsModal';
import { EditorActionBar } from './components/EditorActionBar';
import { EditorHeading } from './components/EditorHeading';
import { EditorTabs } from './components/EditorTabs';
import { EmptyWorkspace } from './components/EmptyWorkspace';
import { ExportTab } from './components/ExportTab';
import { JobsPage } from './components/JobsPage';
import { PlanReviewModal } from './components/PlanReviewModal';
import { PreviewTab } from './components/PreviewTab';
import { SegmentsTab } from './components/SegmentsTab';
import { Sidebar } from './components/Sidebar';
import { TagsTab } from './components/TagsTab';
import { TimestampsModal } from './components/TimestampsModal';
import { Topbar } from './components/Topbar';
import { UrlModal } from './components/UrlModal';
import { desktop } from './desktop';
import { initialEditor } from './editor';
import { time, mergeJob, errorText, active } from './lib';
import type {
  Diagnostics,
  Edit,
  EditorState,
  Job,
  Page,
  Plan,
  Preset,
  Segment,
  Source,
  Tab,
} from './types';

const MEDIA_EXTENSIONS = 'mp3 m4a aac flac wav ogg opus mp4 mkv mov webm avi aiff wma'.split(' ');

export default function App() {
  // Library
  const [sources, setSources] = useState<Source[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [editors, setEditors] = useState<Record<string, EditorState>>({});
  const [jobs, setJobs] = useState<Job[]>([]);

  // Navigation and status
  const [page, setPage] = useState<Page>('editor');
  const [tab, setTab] = useState<Tab>('preview');
  const [ready, setReady] = useState(false);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  // Dialogs
  const [urlOpen, setUrlOpen] = useState(false);
  const [url, setUrl] = useState('');
  const [pasteOpen, setPasteOpen] = useState(false);
  const [timestamps, setTimestamps] = useState('');
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
  const [diagnostics, setDiagnostics] = useState<Diagnostics | null>(null);

  // Export settings
  const [preset, setPreset] = useState<Preset>('mp3');
  const [bitrate, setBitrate] = useState(320);
  const [destination, setDestination] = useState('');
  const [formatId, setFormatId] = useState('');
  const [independent, setIndependent] = useState(false);
  const [plans, setPlans] = useState<Plan[]>([]);

  // Preview
  const [previewError, setPreviewError] = useState(false);
  const [previewPath, setPreviewPath] = useState<string | null>(null);

  const loaded = useRef(false);

  const source = sources.find((s) => s.id === selected[0]);
  const editor = source ? (editors[source.id] ?? initialEditor(source)) : null;
  const chosen = sources.filter((s) => selected.includes(s.id));
  const activeJobs = jobs.filter(active);
  const totalRegions = chosen.reduce((n, s) => n + (editors[s.id]?.segments.length ?? 1), 0);
  const work = useCallback(async (label: string, fn: () => Promise<void>) => {
    setError('');
    setBusy(label);
    try {
      await fn();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy('');
    }
  }, []);
  useEffect(() => {
    if (!desktop) {
      setReady(true);
      return;
    }
    let disposed = false;
    const off: (() => void)[] = [];
    (async () => {
      for (const event of ['job_state_changed', 'stage_progress', 'artifact_ready', 'job_error']) {
        const unlisten = await listen<Job>(event, (e) =>
          setJobs((old) => mergeJob(old, e.payload)),
        );
        if (disposed) unlisten();
        else off.push(unlisten);
      }
      const [ss, jj, project] = await Promise.all([
        invoke<Source[]>('list_sources'),
        invoke<Job[]>('list_jobs'),
        invoke<{
          editors: Record<string, EditorState>;
          destination: string;
          preset: Preset;
          bitrate: number;
        } | null>('get_project', { id: 'workspace' }),
      ]);
      if (disposed) return;
      setSources(ss);
      setSelected(ss.length ? [ss[0].id] : []);
      setJobs((old) => jj.reduce(mergeJob, old));
      setEditors(
        Object.fromEntries(ss.map((s) => [s.id, project?.editors[s.id] ?? initialEditor(s)])),
      );
      if (project) {
        setDestination(project.destination ?? '');
        setPreset(project.preset ?? 'mp3');
        setBitrate(project.bitrate ?? 320);
      }
      loaded.current = true;
      setReady(true);
      const unlisten = await getCurrentWindow().onDragDropEvent((e) => {
        if (e.payload.type === 'drop') void importPaths(e.payload.paths);
      });
      if (disposed) unlisten();
      else off.push(unlisten);
      const close = await getCurrentWindow().onCloseRequested(async (e) => {
        e.preventDefault();
        const current = await invoke<Job[]>('list_jobs');
        if (current.some(active)) {
          const yes = await confirm(
            'Work is still running. Cancel active jobs and close? Completed files will be kept.',
            {
              title: 'Jobs in progress',
              kind: 'warning',
              okLabel: 'Cancel jobs and close',
              cancelLabel: 'Continue working',
            },
          );
          if (!yes) return;
          try {
            await invoke('shutdown_jobs');
          } catch (e) {
            setError(errorText(e));
            return;
          }
        }
        await getCurrentWindow().destroy();
      });
      if (disposed) close();
      else off.push(close);
    })().catch((e) => {
      setError(errorText(e));
      setReady(true);
    });
    return () => {
      disposed = true;
      off.forEach((f) => f());
    };
    // Subscriptions must be established once, before any enqueue.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(() => {
    if (!desktop || !loaded.current) return;
    const timer = setTimeout(() => {
      invoke('save_project', {
        project: {
          id: 'workspace',
          schema_version: 1,
          name: 'My workspace',
          editors,
          destination,
          preset,
          bitrate,
        },
      }).catch((e) => setError(errorText(e)));
    }, 500);
    return () => clearTimeout(timer);
  }, [editors, destination, preset, bitrate]);
  useEffect(() => {
    setPlans([]);
  }, [editors, destination, preset, bitrate, selected, independent, formatId]);
  useEffect(() => {
    setPreviewError(false);
    setPreviewPath(null);
    setFormatId('');
  }, [source?.id]);
  async function importPaths(paths: string[]) {
    await work('Inspecting media', async () => {
      const added: Source[] = [];
      const failures: string[] = [];
      for (const path of paths) {
        try {
          added.push(await invoke<Source>('inspect_file', { path }));
        } catch (e) {
          failures.push(path.split(/[\\/]/).pop() + ': ' + errorText(e));
        }
      }
      setSources((old) => [...old, ...added]);
      setEditors((old) => ({
        ...old,
        ...Object.fromEntries(added.map((s) => [s.id, initialEditor(s)])),
      }));
      if (added.length) setSelected(added.map((s) => s.id));
      setPage('editor');
      setTab('preview');
      if (failures.length) setError(failures.join('\n'));
    });
  }
  async function pickFiles() {
    if (!desktop) {
      setNotice('Open the desktop app with npm run desktop to import and process local media.');
      return;
    }
    const paths = await open({
      multiple: true,
      title: 'Import media',
      filters: [
        {
          name: 'Audio & video',
          extensions: MEDIA_EXTENSIONS,
        },
        { name: 'All files', extensions: ['*'] },
      ],
    });
    if (paths) await importPaths(Array.isArray(paths) ? paths : [paths]);
  }
  async function importUrl() {
    await work('Inspecting URL', async () => {
      if (!desktop) throw new Error('URL inspection is available in the desktop app.');
      const s = await invoke<Source>('inspect_url', { url });
      setSources((old) => [...old, s]);
      setEditors((old) => ({ ...old, [s.id]: initialEditor(s) }));
      setSelected([s.id]);
      setUrlOpen(false);
      setUrl('');
      setPage('editor');
      setTab('preview');
    });
  }
  function patchEditor(patch: Partial<EditorState>) {
    if (!source) return;
    setEditors((old) => ({
      ...old,
      [source.id]: { ...(old[source.id] ?? initialEditor(source)), ...patch },
    }));
  }
  function updateSegment(id: string, patch: Partial<Segment>) {
    if (editor)
      patchEditor({ segments: editor.segments.map((s) => (s.id === id ? { ...s, ...patch } : s)) });
  }
  function updateTag(key: string, edit: Edit) {
    setEditors((old) => {
      const next = { ...old };
      for (const s of chosen) {
        const e = next[s.id] ?? initialEditor(s);
        next[s.id] = { ...e, tags: { ...e.tags, [key]: edit } };
      }
      return next;
    });
  }
  function tagValue(key: string) {
    const values = chosen.map((s) => {
      const edit = editors[s.id]?.tags[key];
      return edit?.action === 'set'
        ? edit.value
        : edit?.action === 'remove'
          ? ''
          : (s.tags[key] ?? '');
    });
    return values.every((v) => v === values[0])
      ? { value: values[0] ?? '', mixed: false }
      : { value: '', mixed: true };
  }
  function setArtwork(edit: Edit) {
    setEditors((old) => {
      const next = { ...old };
      for (const s of chosen) next[s.id] = { ...(next[s.id] ?? initialEditor(s)), artwork: edit };
      return next;
    });
  }
  async function pickArtwork() {
    await work('Preparing artwork', async () => {
      const path = await open({
        multiple: false,
        filters: [{ name: 'Cover art', extensions: ['png', 'jpg', 'jpeg'] }],
      });
      if (typeof path === 'string') {
        const managed = await invoke<string>('import_artwork', { path });
        setArtwork({ action: 'set', value: managed });
      }
    });
  }
  async function parseSegments() {
    if (!source?.duration_us) return;
    await work('Parsing timestamps', async () => {
      const result = await invoke<{ segments: Segment[]; unused_intro_us: number }>(
        'parse_segments',
        { text: timestamps, durationUs: source.duration_us },
      );
      patchEditor({ segments: result.segments });
      setPasteOpen(false);
      setNotice(
        result.unused_intro_us
          ? time(result.unused_intro_us) + ' of introduction is not included.'
          : result.segments.length + ' regions ready.',
      );
    });
  }
  async function review() {
    await work('Preparing export plan', async () => {
      if (!destination) throw new Error('Choose an output folder first.');
      const result: Plan[] = [];
      for (const s of chosen) {
        const e = editors[s.id] ?? initialEditor(s);
        result.push(
          await invoke<Plan>('plan_export', {
            request: {
              source_id: s.id,
              destination,
              preset,
              bitrate,
              segments: e.segments,
              independent_clips: independent,
              tags: e.tags,
              artwork: e.artwork,
              format_id: chosen.length === 1 && formatId ? formatId : null,
            },
          }),
        );
      }
      setPlans(result);
    });
  }
  async function enqueue() {
    await work('Adding to queue', async () => {
      const pending = [...plans];
      while (pending.length) {
        const plan = pending[0];
        const job = await invoke<Job>('enqueue_job', { planId: plan.id, planHash: plan.hash });
        setJobs((old) => mergeJob(old, job));
        pending.shift();
        setPlans([...pending]);
      }
      setPage('jobs');
    });
  }
  async function refreshDiagnostics() {
    setDiagnosticsOpen(true);
    if (desktop)
      await work('Checking dependencies', async () =>
        setDiagnostics(await invoke<Diagnostics>('get_diagnostics')),
      );
  }

  const artwork = editor?.artwork;

  return (
    <div className="app-shell">
      <Sidebar
        page={page}
        setPage={setPage}
        activeJobs={activeJobs}
        sources={sources}
        selected={selected}
        setSelected={setSelected}
        pickFiles={pickFiles}
        refreshDiagnostics={refreshDiagnostics}
      />
      <main>
        <Topbar page={page} setUrlOpen={setUrlOpen} pickFiles={pickFiles} />
        {!desktop && (
          <div className="browser-banner">
            <Info size={16} />
            <span>Interface preview · File processing runs in the desktop app.</span>
            <code>npm run desktop</code>
          </div>
        )}
        {(error || notice) && (
          <div
            className={'notification ' + (error ? 'error' : '')}
            role={error ? 'alert' : 'status'}
          >
            {error ? <AlertCircle size={18} /> : <CheckCircle2 size={18} />}
            <span>{error || notice}</span>
            <button
              className="icon-button"
              aria-label="Dismiss message"
              onClick={() => {
                setError('');
                setNotice('');
              }}
            >
              <X size={16} />
            </button>
          </div>
        )}
        {page === 'editor' ? (
          <>
            <EditorHeading source={source} chosen={chosen} totalRegions={totalRegions} />
            {!source ? (
              <EmptyWorkspace pickFiles={pickFiles} />
            ) : (
              <>
                <EditorTabs tab={tab} setTab={setTab} editor={editor} />
                <section className="editor-content">
                  {tab === 'preview' && (
                    <PreviewTab
                      source={source}
                      previewPath={previewPath}
                      setPreviewPath={setPreviewPath}
                      previewError={previewError}
                      setPreviewError={setPreviewError}
                      busy={busy}
                      work={work}
                      setTab={setTab}
                    />
                  )}
                  {tab === 'segments' && editor && (
                    <SegmentsTab
                      source={source}
                      editor={editor}
                      patchEditor={patchEditor}
                      updateSegment={updateSegment}
                      setPasteOpen={setPasteOpen}
                      independent={independent}
                      setIndependent={setIndependent}
                    />
                  )}
                  {tab === 'tags' && (
                    <TagsTab
                      chosen={chosen}
                      editors={editors}
                      artwork={artwork}
                      tagValue={tagValue}
                      updateTag={updateTag}
                      setArtwork={setArtwork}
                      pickArtwork={pickArtwork}
                    />
                  )}
                  {tab === 'export' && (
                    <ExportTab
                      source={source}
                      chosen={chosen}
                      preset={preset}
                      setPreset={setPreset}
                      bitrate={bitrate}
                      setBitrate={setBitrate}
                      formatId={formatId}
                      setFormatId={setFormatId}
                      destination={destination}
                      setDestination={setDestination}
                      totalRegions={totalRegions}
                      work={work}
                    />
                  )}
                </section>
                <EditorActionBar
                  tab={tab}
                  setTab={setTab}
                  busy={busy}
                  totalRegions={totalRegions}
                  review={review}
                />
              </>
            )}
          </>
        ) : (
          <JobsPage
            jobs={jobs}
            activeJobs={activeJobs}
            setPage={setPage}
            setJobs={setJobs}
            work={work}
          />
        )}
      </main>
      {busy && (
        <div className="busy-indicator" role="status">
          <LoaderCircle size={17} className="spin" />
          {busy}…
        </div>
      )}
      {urlOpen && (
        <UrlModal
          url={url}
          setUrl={setUrl}
          setUrlOpen={setUrlOpen}
          importUrl={importUrl}
          busy={busy}
          error={error}
        />
      )}
      {pasteOpen && (
        <TimestampsModal
          timestamps={timestamps}
          setTimestamps={setTimestamps}
          setPasteOpen={setPasteOpen}
          parseSegments={parseSegments}
          busy={busy}
          error={error}
        />
      )}
      {plans.length > 0 && (
        <PlanReviewModal
          plans={plans}
          setPlans={setPlans}
          enqueue={enqueue}
          busy={busy}
          error={error}
        />
      )}
      {diagnosticsOpen && (
        <DiagnosticsModal
          diagnostics={diagnostics}
          setDiagnosticsOpen={setDiagnosticsOpen}
          refreshDiagnostics={refreshDiagnostics}
          error={error}
        />
      )}
      {!ready && (
        <div className="loading-screen">
          <LoaderCircle className="spin" /> Opening workspace…
        </div>
      )}
    </div>
  );
}
