import { expect, test } from "@playwright/test";

test("current app renders tracks from the API", async ({ page }) => {
  await page.goto("/");

  await expect(page.locator('[data-testid="track-item"]').first()).toBeVisible({
    timeout: 10000,
  });
  await expect(page.getByText("No tracks available")).toHaveCount(0);
});
