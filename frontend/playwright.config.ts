import { defineConfig } from '@playwright/test';

export default defineConfig({
	webServer: {
		command: 'VITE_MOCK_API=true npm run build && VITE_MOCK_API=true npm run preview',
		port: 4173,
		reuseExistingServer: !process.env.CI
	},
	testDir: 'e2e',
	use: {
		// Use prepared request context with pre-configured routes for all tests
		baseURL: 'http://localhost:4173'
	},
	// Add proper timing and retry configurations
	timeout: 30000,
	expect: {
		timeout: 5000
	},
	retries: process.env.CI ? 2 : 0,
	workers: process.env.CI ? 1 : undefined
});
