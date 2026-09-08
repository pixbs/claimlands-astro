import { test, expect } from '@playwright/test';

test('stable foundation view', async ({ page }, testInfo) => {
  await page.goto('/?backend=webgl&frequency=8&seed=63352');
  await expect(page.locator('#game')).toHaveAttribute('data-ready', 'true');
  await expect(page.locator('#game')).toHaveAttribute('data-backend', 'Gl');
  await page.mouse.move(1270, 790);
  // Crop excludes the revision label and egui text, keeping this baseline about
  // the planet rendered with the pinned browser's software WebGL backend.
  const clip = { x: 350, y: 100, width: 850, height: 600 };
  await expect(page).toHaveScreenshot('foundation.png', {
    clip,
    maxDiffPixelRatio: 0.001,
  });
  // Preserve evidence on success too; Playwright otherwise retains only failures.
  await page.screenshot({ path: testInfo.outputPath('foundation-actual.png'), clip });
});
