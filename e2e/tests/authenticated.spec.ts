import { test, expect } from '@playwright/test';
import { promises as fs } from 'fs';
import { join } from 'path';
import { parseFile } from 'music-metadata';

async function getID3Version(filePath: string): Promise<string> {
  try {
    const metadata = await parseFile(filePath);
    const format = metadata.format;
    if (format.tagTypes?.includes('ID3v2.4')) return '2.4';
    if (format.tagTypes?.includes('ID3v2.3')) return '2.3';
    if (format.tagTypes?.includes('ID3v2.2')) return '2.2';
    return 'none';
  } catch {
    return 'error';
  }
}

test.describe('Authenticated User Flows', () => {
  test('playback controls work correctly', async ({ page }) => {
    await page.goto('/');

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

    const musicDir = join(process.cwd(), 'music');
    const sourceFile = join(musicDir, 'The Midnight Echoes - Digital Dreams.wav');
    const originalFile = join(process.cwd(), 'original', 'The Midnight Echoes - Digital Dreams.wav');

    const preVersion = await getID3Version(sourceFile);
    expect(preVersion).toBe('2.4');

    await fs.unlink(sourceFile);
    await expect(midnightEchoesTrack).not.toBeVisible({ timeout: 10000 });
    await fs.copyFile(originalFile, sourceFile);

    await expect(midnightEchoesTrack).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(5000);

    const postVersion = await getID3Version(sourceFile);
    expect(postVersion).toBe('2.3');

    await page.getByRole('button', { name: 'Next Track' }).click();

    await expect(page.locator('.fixed.bottom-0')).toContainText('The Midnight Echoes - Digital Dreams');
    await expect(page.locator('audio')).toHaveJSProperty('paused', false);
    await expect(midnightEchoesTrack).toHaveClass(/bg-secondary/);
    await expect(lunaTrack).not.toHaveClass(/bg-secondary/);
  });
});
