import { describe, it, expect } from 'vitest';
import { time, parseTime, mergeJob } from './lib';
import type { Job } from './types';
describe('editor timestamps', () => {
  it('preserves long recordings', () => {
    expect(parseTime('1:02:03')).toBe(3723e6);
    expect(time(3723e6)).toBe('1:02:03');
  });
  it('rejects malformed fields', () => {
    for (const s of ['1:60', '-1:00', '1:2', 'abc', '1:00:99'])
      expect(() => parseTime(s)).toThrow();
  });
});
describe('job event reconciliation', () => {
  const job = { id: 'one', attempt: 2, sequence: 10, created_at: 1 } as Job;
  it('ignores late events', () => {
    expect(mergeJob([job], { ...job, sequence: 9 })[0]).toBe(job);
    expect(mergeJob([job], { ...job, attempt: 1, sequence: 100 })[0]).toBe(job);
  });
  it('accepts new attempts', () =>
    expect(mergeJob([job], { ...job, attempt: 3, sequence: 1 })[0].attempt).toBe(3));
});
