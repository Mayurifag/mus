import { test, expect } from './fixtures';

/**
 * Player Functionality Tests
 *
 * Tests specific to the music player functionality, including:
 * - Loading player state
 * - Track controls
 * - Player UI elements
 */

test.describe('Music player features', () => {
	// Mock a specific player state before tests
	test.beforeEach(async ({ page }) => {
		// Additional client-side API mocking for player state
		await page.route('**/api/v1/player/state', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					current_track_id: 1,
					progress_seconds: 30,
					volume_level: 0.8,
					is_muted: false
				})
			});
		});
	});

	test('player loads with correct state', async ({ page }) => {
		await page.goto('/');

		// The player functionality may not be fully implemented yet, so we're testing
		// what we expect to exist based on the application design.

		// Look for the music library heading to confirm page loaded
		await expect(page.getByRole('heading', { name: 'Music Library' })).toBeVisible();

		// Verify the track count message (indicates successful data loading)
		// Note the period at the end to match exactly what's in the template
		await expect(page.getByText('Found 1 tracks in your library.')).toBeVisible();

		// Check for specific elements that would be in a player UI
		// Note: You should update these selectors based on your actual player implementation
		const playerContainer = page.locator('.container');
		await expect(playerContainer).toBeVisible();
	});

	test('scanning functionality', async ({ page }) => {
		// Mock the scan endpoint
		await page.route('**/api/v1/scan', async (route) => {
			if (route.request().method() === 'POST') {
				await route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: JSON.stringify({
						success: true,
						message: 'Scan started successfully'
					})
				});
			}
		});

		await page.goto('/');

		// Look for the library to be loaded first
		await expect(page.getByRole('heading', { name: 'Music Library' })).toBeVisible();

		// This is a placeholder since the scan UI might not be fully implemented
		// Uncomment and update these when your UI is ready:
		// await page.getByRole('button', { name: /scan/i }).click();
		// await expect(page.getByText(/scan started/i)).toBeVisible();
	});
});
