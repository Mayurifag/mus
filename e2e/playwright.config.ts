import { defineConfig, devices } from '@playwright/test';

const useForChromium = {
  ...devices['Desktop Chrome'],
  launchOptions: {
    args: ['--disable-web-security', '--disable-features=VizDisplayCompositor'],
  },
};

export default defineConfig({
  testDir: './tests',
  outputDir: 'test-results',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  timeout: 30 * 1000,
  expect: {
    timeout: 10 * 1000,
  },
  reporter: [
    ['html', { open: 'never' }],
    ['junit', { outputFile: 'test-results/junit.xml' }],
    process.env.CI ? ['github'] : ['list'],
  ],
  use: {
    baseURL: 'http://localhost:4124',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    actionTimeout: 15 * 1000,
    navigationTimeout: 30 * 1000,
  },

  projects: [
    {
      name: 'setup',
      testMatch: /.*\.setup\.ts/,
      use: useForChromium,
    },
    {
      name: 'unauthenticated-tests',
      testMatch: /.*auth\.spec\.ts/,
      use: {
        ...useForChromium,
        storageState: { cookies: [], origins: [] },
      },
    },
    {
      name: 'authenticated-tests',
      testMatch: /.*authenticated\.spec\.ts/,
      dependencies: ['setup'],
      use: {
        ...useForChromium,
        storageState: 'playwright/.auth/user.json',
      },
    },
  ],
});
