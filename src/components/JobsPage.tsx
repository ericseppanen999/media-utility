import { invoke } from '@tauri-apps/api/core';
import {
  ArrowUpRight,
  Check,
  CheckCircle2,
  ChevronRight,
  Download,
  FileAudio,
  LoaderCircle,
  RefreshCw,
  X,
} from 'lucide-react';
import { active, mergeJob } from '../lib';
import type { Job, Page, Work } from '../types';

interface JobsPageProps {
  jobs: Job[];
  activeJobs: Job[];
  setPage: (page: Page) => void;
  setJobs: React.Dispatch<React.SetStateAction<Job[]>>;
  work: Work;
}

export function JobsPage({ jobs, activeJobs, setPage, setJobs, work }: JobsPageProps) {
  return (
    <>
      <section className="page-heading">
        <div>
          <h1>Jobs</h1>
          <p>Export progress and history.</p>
        </div>
        <span className="pill">{activeJobs.length} active</span>
      </section>
      <div className="jobs-list">
        {!jobs.length ? (
          <div className="jobs-empty">
            <Download size={36} />
            <h2>No jobs</h2>
            <p>Start an export from the editor.</p>
            <button className="button secondary" onClick={() => setPage('editor')}>
              Back to editor <ChevronRight size={16} />
            </button>
          </div>
        ) : (
          jobs.map((job) => <JobCard key={job.id} job={job} work={work} setJobs={setJobs} />)
        )}
      </div>
    </>
  );
}

interface JobCardProps {
  job: Job;
  setJobs: React.Dispatch<React.SetStateAction<Job[]>>;
  work: Work;
}

function JobCard({ job, setJobs, work }: JobCardProps) {
  return (
    <div className="job-card">
      <div className={'job-icon ' + job.state}>
        {job.state === 'succeeded' ? (
          <CheckCircle2 size={24} />
        ) : active(job) ? (
          <LoaderCircle className="spin" size={24} />
        ) : (
          <FileAudio size={24} />
        )}
      </div>
      <div className="job-body">
        <div className="job-title">
          <h3>{job.title}</h3>
          <span className={'job-state ' + job.state}>{job.state}</span>
        </div>
        <p>
          {job.stage} <span>·</span> Attempt {job.attempt} <span>·</span> {job.artifacts.length}{' '}
          completed
        </p>
        {active(job) && (
          <div className={'progress-track ' + (job.progress === null ? 'indeterminate' : '')}>
            <span
              style={{
                width: job.progress === null ? '35%' : job.progress * 100 + '%',
              }}
            />
          </div>
        )}
        {job.error && <p className="job-error">{job.error.message}</p>}
        <div className="artifact-list">
          {job.artifacts.map((a) => (
            <button
              key={a.id}
              onClick={() =>
                void work('Revealing output', async () => {
                  await invoke('reveal_output', { jobId: job.id, artifactId: a.id });
                })
              }
            >
              <Check size={13} />
              <span>{a.path.split(/[\\/]/).pop()}</span>
              <ArrowUpRight size={13} />
            </button>
          ))}
        </div>
        <div className="job-destination">{job.destination}</div>
      </div>
      <div className="job-actions">
        {active(job) ? (
          <button
            className="button secondary compact"
            onClick={() =>
              void work('Cancelling job', async () => {
                await invoke('cancel_job', { jobId: job.id });
              })
            }
          >
            <X size={14} /> Cancel
          </button>
        ) : (
          ['failed', 'cancelled', 'interrupted'].includes(job.state) && (
            <button
              className="button secondary compact"
              onClick={() =>
                void work('Retrying job', async () => {
                  const j = await invoke<Job>('retry_job', { jobId: job.id });
                  setJobs((old) => mergeJob(old, j));
                })
              }
            >
              <RefreshCw size={14} /> Retry
            </button>
          )
        )}
      </div>
    </div>
  );
}
