import { test as base } from '@playwright/test';

// Extend the Playwright test with custom fixtures
export const test = base.extend({
	// Setup logic before each test
	page: async ({ page }, use) => {
		// Intercept console errors to help with debugging
		page.on('console', (msg) => {
			if (msg.type() === 'error') {
				console.error(`Browser console error: ${msg.text()}`);
			}
		});

		// Intercept failed network requests
		page.on('requestfailed', (request) => {
			console.error(`Failed request: ${request.url()}`);
		});

		// Could add more setup logic here
		await use(page);

		// Could add cleanup logic here
	}
});

// Re-export expect so tests only need to import from fixtures
export { expect } from '@playwright/test';
