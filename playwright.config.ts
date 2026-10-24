import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/browser',
  forbidOnly: true,
  retries: 0,
  workers: 2,
  reporter: 'list',
  outputDir: 'test-results',
  snapshotPathTemplate: '{testDir}/__screenshots__/{projectName}/{testFilePath}/{arg}{ext}',
  use: {
    browserName: 'chromium',
    baseURL: 'http://127.0.0.1:4175',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [
    { name: 'chromium-2458x1358-dpr1', use: { viewport: { width: 2458, height: 1358 }, deviceScaleFactor: 1 } },
    { name: 'chromium-2458x1358-dpr2', use: { viewport: { width: 2458, height: 1358 }, deviceScaleFactor: 2 } },
    { name: 'chromium-1280x800-dpr1', use: { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 } },
    { name: 'chromium-1280x800-dpr2', use: { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 } },
  ],
  webServer: {
    command: 'npm run build && npm run preview -- --host 127.0.0.1 --port 4175 --strictPort',
    url: 'http://127.0.0.1:4175',
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
