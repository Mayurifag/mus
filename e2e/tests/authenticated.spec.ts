import { test, expect } from '@playwright/test';

test.describe('Authenticated User Flows', () => {
  test('playback controls work correctly', async ({ page }) => {
    await page.goto('/');

    const track1 = page.locator('#track-item-1');
    const track2 = page.locator('#track-item-2');

    await expect(track1).toBeVisible({ timeout: 5000 });
    await expect(track2).toBeVisible();
    await expect(page.locator('audio')).toHaveJSProperty('paused', true);

    await track2.click();

    await expect(page.locator('.fixed.bottom-0')).toContainText('Luna Solaris - Cosmic Wanderer');
    await expect(page.locator('audio')).toHaveJSProperty('paused', false);
    await expect(track2).toHaveClass(/bg-secondary/);
    await expect(track1).not.toHaveClass(/bg-secondary/);

    await page.getByRole('button', { name: 'Next Track' }).click();

    await expect(page.locator('.fixed.bottom-0')).toContainText('The Midnight Echoes - Digital Dreams');
    await expect(page.locator('audio')).toHaveJSProperty('paused', false);
    await expect(track1).toHaveClass(/bg-secondary/);
    await expect(track2).not.toHaveClass(/bg-secondary/);
  });
});
