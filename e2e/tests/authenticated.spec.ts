import { test, expect } from '@playwright/test';
import { promises as fs } from 'fs';
import { join } from 'path';

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

    // Test filesystem events before clicking Next Track
    const musicDir = join(process.cwd(), 'music');
    const sourceFile = join(musicDir, 'The Midnight Echoes - Digital Dreams.wav');
    const tempFile = join(process.cwd(), 'The Midnight Echoes - Digital Dreams.wav');

    let fileMoved = false;
    try {
      // Move file out of music directory
      await fs.rename(sourceFile, tempFile);
      fileMoved = true;

      // Wait for file watcher to detect deletion and check track-item-1 is gone
      await expect(track1).not.toBeVisible({ timeout: 10000 });

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
    await expect(track1).toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: 'Next Track' }).click();

    await expect(page.locator('.fixed.bottom-0')).toContainText('The Midnight Echoes - Digital Dreams');
    await expect(page.locator('audio')).toHaveJSProperty('paused', false);
    await expect(track1).toHaveClass(/bg-secondary/);
    await expect(track2).not.toHaveClass(/bg-secondary/);
  });
});
