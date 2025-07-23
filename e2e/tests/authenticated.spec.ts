import { test, expect } from '@playwright/test';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

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
    const containerName = 'mus-e2e-test';
    const sourceFile = '/app_data/music/The Midnight Echoes - Digital Dreams.wav';
    const tempFile = '/app_data/The Midnight Echoes - Digital Dreams.wav';

    // Move file out of music directory using docker exec
    await execAsync(`docker exec ${containerName} mv "${sourceFile}" "${tempFile}"`);

    // Wait for file watcher to detect deletion and check track-item-1 is gone
    await expect(track1).not.toBeVisible({ timeout: 10000 });

    // Move file back to music directory using docker exec
    await execAsync(`docker exec ${containerName} mv "${tempFile}" "${sourceFile}"`);

    // Wait for file watcher to detect creation and check track-item-1 is back
    await expect(track1).toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: 'Next Track' }).click();

    await expect(page.locator('.fixed.bottom-0')).toContainText('The Midnight Echoes - Digital Dreams');
    await expect(page.locator('audio')).toHaveJSProperty('paused', false);
    await expect(track1).toHaveClass(/bg-secondary/);
    await expect(track2).not.toHaveClass(/bg-secondary/);
  });
});
