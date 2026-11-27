import { expect, test } from "@playwright/test";

test("renders tracks and one set of playback controls at both widths", async ({
  page,
}) => {
  await page.goto("/");

  await expect(page.locator('[data-testid="track-item"]').first()).toBeVisible({
    timeout: 10000,
  });
  await expect(page.getByText("No tracks available")).toHaveCount(0);

  const nextTrack = page
    .locator("[data-player-footer]")
    .getByRole("button", { name: "Next Track", exact: true });
  await expect(nextTrack).toHaveCount(1);
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(nextTrack).toHaveCount(1);
});
