import { test, expect } from '@playwright/test';
import { promises as fs } from 'fs';
import { join } from 'path';
import { parseFile } from 'music-metadata';
import { execSync } from 'child_process';

interface MetadataInfo {
  id3Version: string;
  duration: number;
}

async function getMetadataInfo(filePath: string): Promise<MetadataInfo> {
  try {
    const metadata = await parseFile(filePath);
    const format = metadata.format;

    let id3Version = 'none';
    if (format.tagTypes?.includes('ID3v2.4')) id3Version = '2.4';
    else if (format.tagTypes?.includes('ID3v2.3')) id3Version = '2.3';
    else if (format.tagTypes?.includes('ID3v2.2')) id3Version = '2.2';

    const duration = format.duration ? Math.round(format.duration) : 0;

    return { id3Version, duration };
  } catch {
    return { id3Version: 'error', duration: 0 };
  }
}

async function checkCoverFiles(trackId: number): Promise<{ hasSmall: boolean; hasOriginal: boolean }> {
  const containerName = 'mus-e2e-test';
  const smallPath = `/app_data/covers/${trackId}_small.webp`;
  const originalPath = `/app_data/covers/${trackId}_original.webp`;

  const checkFileExists = (filePath: string): boolean => {
    try {
      execSync(`docker exec ${containerName} test -f "${filePath}"`, { stdio: 'ignore' });
      return true;
    } catch (error) {
      return false;
    }
  };

  const hasSmall = checkFileExists(smallPath);
  const hasOriginal = checkFileExists(originalPath);

  return { hasSmall, hasOriginal };
}

async function getFileStats(filePath: string): Promise<{ mtime: number; exists: boolean }> {
  try {
    const stats = await fs.stat(filePath);
    return { mtime: stats.mtimeMs, exists: true };
  } catch {
    return { mtime: 0, exists: false };
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

    const preMetadata = await getMetadataInfo(originalFile);
    const preStats = await getFileStats(originalFile);
    expect(preMetadata.id3Version).toBe('2.4');

    await fs.unlink(sourceFile);
    await expect(midnightEchoesTrack).not.toBeVisible({ timeout: 10000 });
    await fs.copyFile(originalFile, sourceFile);

    await expect(midnightEchoesTrack).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(8000);

    const postMetadata = await getMetadataInfo(sourceFile);
    const postStats = await getFileStats(sourceFile);

    // File has to become ID3v2.3 after processing - before start of tests it was v2.4
    expect(postMetadata.id3Version).toBe('2.3');
    expect(postStats.mtime).not.toBe(preStats.mtime);
    expect(postMetadata.duration).toBeGreaterThan(0);

    const trackId = await midnightEchoesTrack.getAttribute('id');
    const trackIdNumber = parseInt(trackId?.replace('track-item-', '') || '1');

    const coverFiles = await checkCoverFiles(trackIdNumber);
    expect(coverFiles.hasSmall).toBe(true);
    expect(coverFiles.hasOriginal).toBe(true);

    await page.getByRole('button', { name: 'Next Track' }).click();

    await expect(page.locator('.fixed.bottom-0')).toContainText('The Midnight Echoes - Digital Dreams');
    await expect(page.locator('audio')).toHaveJSProperty('paused', false);
    await expect(midnightEchoesTrack).toHaveClass(/bg-secondary/);
    await expect(lunaTrack).not.toHaveClass(/bg-secondary/);
  });
});
