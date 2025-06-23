import { test, expect } from '@playwright/test';

test('authentication flow', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Access Restricted' })).toBeVisible();
  await expect(page).toHaveURL(/.*login\.html/);

  const invalidResponse = await page.goto('/login?token=wrong-token');
  expect(invalidResponse?.status()).toBe(401);

  await page.goto('/login?token=e2e-secret-key');
  await expect(page.locator('#track-item-1')).toBeVisible({ timeout: 5000 });
  await expect(page).not.toHaveURL(/.*login/);
});
