import type { Job } from './types';
export function time(us: number | null): string {
  if (us === null) return 'Unknown';
  const seconds = Math.floor(us / 1e6);
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return h
    ? h + ':' + String(m).padStart(2, '0') + ':' + String(s).padStart(2, '0')
    : m + ':' + String(s).padStart(2, '0');
}
export function parseTime(value: string): number {
  const parts = value.split(':');
  if (
    parts.length < 2 ||
    parts.length > 3 ||
    parts.some((v, i) => !/^\d+$/.test(v) || (i > 0 && (v.length !== 2 || Number(v) > 59)))
  )
    throw new Error('Use M:SS or H:MM:SS');
  const n = parts.reduce((n, p) => n * 60 + Number(p), 0) * 1e6;
  if (!Number.isSafeInteger(n)) throw new Error('Time is too large');
  return n;
}
export function mergeJob(jobs: Job[], job: Job): Job[] {
  const old = jobs.find((j) => j.id === job.id);
  if (
    old &&
    (old.attempt > job.attempt || (old.attempt === job.attempt && old.sequence >= job.sequence))
  )
    return jobs;
  return [job, ...jobs.filter((j) => j.id !== job.id)].sort((a, b) => b.created_at - a.created_at);
}
export function errorText(e: unknown): string {
  if (typeof e === 'object' && e && 'message' in e) return String(e.message);
  return String(e);
}
export const active = (j: Job) => ['queued', 'running', 'cancelling'].includes(j.state);
