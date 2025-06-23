import { test as setup, expect } from '@playwright/test';
import { existsSync } from 'fs';

const authFile = 'playwright/.auth/user.json';

setup('authenticate and save state', async ({ page }) => {
  if (existsSync(authFile)) {
    console.log('Auth file exists, skipping authentication setup');
    return;
  }

  console.log('Auth file not found, performing authentication setup');
  await page.goto('/login?token=e2e-secret-key');
  await expect(page.locator('#track-item-1')).toBeVisible({ timeout: 5000 });
  await page.context().storageState({ path: authFile });
});
