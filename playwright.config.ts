import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/ui',
  timeout: 60000,
  use: {
    channel: 'msedge',
    viewport: { width: 1320, height: 900 },
    baseURL: 'http://127.0.0.1:1420',
  },
  webServer: { command: 'npm run dev', url: 'http://127.0.0.1:1420', reuseExistingServer: true },
  reporter: 'list',
});
