import type { Edit, EditorState, Source } from './types';

export const unchanged: Edit = { action: 'unchanged' };

export const initialEditor = (source: Source): EditorState => ({
  segments: source.duration_us
    ? [{ id: crypto.randomUUID(), start_us: 0, end_us: source.duration_us, title: source.title }]
    : [],
  tags: {},
  artwork: unchanged,
});
