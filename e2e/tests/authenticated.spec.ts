import { test, expect } from '@playwright/test';
import { promises as fs } from 'fs';
import { join } from 'path';

test.describe('Authenticated User Flows', () => {
  test('playback controls work correctly', async ({ page }) => {
    await page.goto('/');

    // Use content-based selectors instead of hard-coded IDs
    const midnightEchoesTrack = page.locator('[data-testid="track-item"]').filter({ hasText: 'The Midnight Echoes' });
    const lunaTrack = page.locator('[data-testid="track-item"]').filter({ hasText: 'Luna Solaris' });

    await expect(midnightEchoesTrack).toBeVisible({ timeout: 5000 });
    await expect(lunaTrack).toBeVisible();
    await expect(page.locator('audio')).toHaveJSProperty('paused', true);

    await lunaTrack.click();

    await expect(page.locator('.fixed.bottom-0')).toContainText('Luna Solaris - Cosmic Wanderer');
    await expect(page.locator('audio')).toHaveJSProperty('paused', false);
    await expect(lunaTrack).toHaveClass(/bg-secondary/);
    await expect(midnightEchoesTrack).not.toHaveClass(/bg-secondary/);

    // Test filesystem events before clicking Next Track
    const musicDir = join(process.cwd(), 'music');
    const sourceFile = join(musicDir, 'The Midnight Echoes - Digital Dreams.wav');
    const tempFile = join(process.cwd(), 'The Midnight Echoes - Digital Dreams.wav');

    let fileMoved = false;
    try {
      // Move file out of music directory
      await fs.rename(sourceFile, tempFile);
      fileMoved = true;

      // Wait for file watcher to detect deletion and check Midnight Echoes track is gone
      await expect(midnightEchoesTrack).not.toBeVisible({ timeout: 10000 });

      // Move file back to music directory
      await fs.rename(tempFile, sourceFile);
      fileMoved = false;

    } finally {
      // Ensure file is always restored to music directory
      if (fileMoved) {
        try {
          await fs.rename(tempFile, sourceFile);
        } catch (error) {
          console.error('Failed to restore file:', error);
        }
      }
    }
    // Wait for file watcher to detect creation and check Midnight Echoes track is back
    await expect(midnightEchoesTrack).toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: 'Next Track' }).click();

    await expect(page.locator('.fixed.bottom-0')).toContainText('The Midnight Echoes - Digital Dreams');
    await expect(page.locator('audio')).toHaveJSProperty('paused', false);
    await expect(midnightEchoesTrack).toHaveClass(/bg-secondary/);
    await expect(lunaTrack).not.toHaveClass(/bg-secondary/);
  });
});
