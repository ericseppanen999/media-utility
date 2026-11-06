import { test, expect } from '@playwright/test';
test('empty workspace and desktop boundary are clear', async ({ page }) => {
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await expect(
    page.getByRole('heading', { name: 'Import media' }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Choose files', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Open the desktop app');
  await page.getByRole('button', { name: 'Paste URL', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await page.screenshot({ path: 'tests/artifacts/workspace.png', fullPage: true });
});
test('native IPC fixture supports editing and reviewed export', async ({ page }) => {
  await page.addInitScript(() => {
    const source = {
      id: 'source-1',
      title: 'Live at the garden',
      path: null,
      url: 'https://www.youtube.com/watch?v=test',
      duration_us: 180000000,
      fingerprint: 'fixture',
      streams: [
        {
          index: 0,
          kind: 'audio',
          codec: 'aac',
          sample_rate: '48000',
          channels: 2,
          width: null,
          height: null,
        },
      ],
      chapters: [],
      tags: { artist: 'Example artist' },
      formats: [],
      thumbnail: null,
    };
    let seq = 0;
    const internal = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
      transformCallback: () => ++seq,
      unregisterCallback: () => {},
      convertFileSrc: (p: string) => p,
      invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === 'list_sources') return [source];
        if (command === 'list_jobs') return [];
        if (command === 'get_project') return null;
        if (command === 'plugin:event|listen') return ++seq;
        if (command === 'plugin:dialog|open') return 'C:\\Exports';
        if (command === 'plan_export') {
          const r = args.request as { segments: { id: string; title: string }[] };
          return {
            id: 'plan-1',
            hash: 'reviewed',
            source,
            request: r,
            copy: false,
            extension: 'mp3',
            explanation: 'Encode directly from original into .mp3.',
            warnings: [],
            dependencies: {},
            outputs: r.segments.map((s) => ({
              segment: s,
              path: 'C:\\Exports\\01 - ' + s.title + '.mp3',
            })),
          };
        }
        if (command === 'enqueue_job')
          return {
            id: 'job-1',
            plan_id: 'plan-1',
            title: source.title,
            state: 'queued',
            stage: 'inspect',
            progress: null,
            attempt: 1,
            sequence: 1,
            error: null,
            artifacts: [],
            created_at: 1,
            destination: 'C:\\Exports',
          };
        return null;
      },
    };
    Object.defineProperty(window, 'isTauri', { value: true });
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: internal });
  });
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await expect(page.getByRole('heading', { name: 'Live at the garden' })).toBeVisible();
  await page.getByRole('tab', { name: 'Segments' }).click();
  await page.getByRole('textbox', { name: 'Region 1 title' }).fill('Opening song');
  await page.getByRole('textbox', { name: 'Region 1 start' }).fill('0:10');
  await page.getByRole('textbox', { name: 'Region 1 end' }).focus();
  await page.getByRole('tab', { name: 'Tags & artwork' }).click();
  await page.getByRole('textbox', { name: 'Album', exact: true }).fill('Garden sessions');
  await page.getByRole('tab', { name: 'Export', exact: true }).click();
  await page.getByRole('button', { name: 'Choose folder' }).click();
  await page.getByRole('button', { name: 'Review 1 output' }).click();
  await expect(page.getByRole('dialog')).toContainText('RE-ENCODE');
  await expect(page.getByRole('dialog')).toContainText('Opening song.mp3');
  await page.screenshot({ path: 'tests/artifacts/export-review.png', fullPage: true });
  await page.getByRole('button', { name: 'Start export' }).click();
  await expect(page.getByRole('heading', { name: 'Jobs' })).toBeVisible();
  await expect(page.getByText('queued', { exact: true })).toBeVisible();
});
