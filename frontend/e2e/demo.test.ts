import { test, expect } from './fixtures';

/**
 * End-to-End Tests for Music Player Application
 *
 * These tests verify the core functionality of the music player application
 * by testing the UI with mocked API responses.
 */

test.describe('Music player application', () => {
	test('home page loads with track data', async ({ page }) => {
		await page.goto('/');

		await expect(page.getByRole('heading', { name: 'Music Library' })).toBeVisible();
		await expect(page.getByText('Found 1 tracks in your library.')).toBeVisible();
	});

	test('client-side API intercepted requests work correctly', async ({ page }) => {
		await page.route('**/api/v1/tracks', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([
					{
						id: 2,
						title: 'Client-side Mocked Track',
						artist: 'Playwright Test',
						duration: 240,
						file_path: '/path/to/mocked_track.mp3',
						added_at: Date.now(),
						has_cover: true,
						cover_small_url: '/api/v1/tracks/2/covers/small.webp',
						cover_original_url: '/api/v1/tracks/2/covers/original.webp'
					}
				])
			});
		});

		await page.goto('/');
		await expect(page.getByText('Found 1 tracks in your library.')).toBeVisible();
	});

	test('basic page navigation and layout', async ({ page }) => {
		await page.goto('/');

		await expect(page.getByRole('heading', { name: 'Music Library' })).toBeVisible();
		const mainContainer = page.locator('.container');
		await expect(mainContainer).toBeVisible();
		await expect(page).toHaveURL('/');
	});
});
