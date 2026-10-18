import { defineConfig } from '@playwright/test';
import { randomUUID } from 'node:crypto';

export default defineConfig({
  testDir: './apps/web/tests',
  testMatch: '**/*.spec.ts',
  workers: 1,
  timeout: 60_000,
  use: {
    baseURL: 'http://127.0.0.1:1421',
    viewport: { width: 1440, height: 1000 },
    permissions: ['clipboard-read', 'clipboard-write'],
    launchOptions: {
      executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH,
      args: process.env.CI ? [] : ['--no-sandbox', '--disable-dev-shm-usage'],
    },
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: [
    {
      command: `cargo run -p knowtosay --bin knowtosay-api -- --data-dir .local/e2e-${randomUUID()} --port 4175 --no-lan`,
      url: 'http://127.0.0.1:4175/api/v1/snapshot',
      timeout: 120_000,
    },
    {
      command: 'bun run --cwd apps/web dev --port 1421',
      url: 'http://127.0.0.1:1421',
      env: { KNOWTOSAY_API_PORT: '4175' },
      timeout: 60_000,
    },
  ],
});
