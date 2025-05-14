import { test, expect } from './fixtures';

/**
 * Error Handling Tests
 *
 * Tests that focus on how the application handles error conditions:
 * - API failure cases
 * - Empty states
 * - Network errors
 */

test.describe('Application error handling', () => {
	test('handles non-existent player state', async ({ page }) => {
		// Mock 404 response for player state (new user case)
		await page.route('**/api/v1/player/state', async (route) => {
			await route.fulfill({
				status: 404,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'Not found' })
			});
		});

		await page.goto('/');

		// The page should still load normally with track data
		await expect(page.getByRole('heading', { name: 'Music Library' })).toBeVisible();

		// We should still see track information from the mocked tracks API
		await expect(page.getByText('Found 1 tracks in your library.')).toBeVisible();
	});
});
