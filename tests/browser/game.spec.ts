import { test, expect, type Page, type TestInfo } from '@playwright/test';

async function openGame(page: Page, testInfo: TestInfo, frequency = 8, seed = 63352) {
  const backend = testInfo.project.metadata.backend;
  expect(['webgl', 'webgpu']).toContain(backend);
  await page.goto(`/?backend=${backend}&frequency=${frequency}&seed=${seed}`);
  const game = page.locator('#game');
  await expect(game).toHaveAttribute('data-ready', 'true');
  await expect(game).not.toHaveAttribute('data-error');
  await expect(game).toHaveAttribute('data-backend', backend === 'webgl' ? 'Gl' : 'BrowserWebGpu');
  await expect(game).toHaveAttribute('data-tile-count', String(10 * frequency * frequency + 2));
  if (process.env.EXPECTED_REVISION) {
    await expect(game).toHaveAttribute('data-revision', process.env.EXPECTED_REVISION);
  }
  return game;
}

test('deployed game renders and responds', async ({ page }, testInfo) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  const game = await openGame(page, testInfo);
  await page.screenshot({ path: testInfo.outputPath('foundation-review.png'), clip: { x: 350, y: 100, width: 850, height: 600 } });
  const before = await game.getAttribute('data-rotation');
  await page.mouse.move(740, 400);
  await page.mouse.down();
  await page.mouse.move(850, 450, { steps: 8 });
  await page.mouse.up();
  await expect(game).not.toHaveAttribute('data-rotation', before!);
  await page.mouse.click(640, 400);
  await expect(game).toHaveAttribute('data-selected-tile', /\d+/);
  const distance = await game.getAttribute('data-distance');
  await page.mouse.wheel(0, -120);
  await expect(game).not.toHaveAttribute('data-distance', distance!);
  await page.keyboard.press('n');
  await expect(game).toHaveAttribute('data-seed', '63353');
  await page.keyboard.press('0');
  await expect(game).toHaveAttribute('data-seed', '0');
  await page.keyboard.press('-');
  await expect(game).toHaveAttribute('data-frequency', '7');
  await expect(game).toHaveAttribute('data-tile-count', '492');
  await expect(game).not.toHaveAttribute('data-error');
  expect(errors).toEqual([]);
  await page.screenshot({ path: testInfo.outputPath('game-review.png') });
});

test('deployed game survives repeated regeneration and viewport changes', async ({ page }, testInfo) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  const game = await openGame(page, testInfo, 2, 0);
  for (let seed = 1; seed <= 6; seed++) {
    await page.keyboard.press('n');
    await expect(game).toHaveAttribute('data-seed', String(seed));
    await page.keyboard.press('=');
    await expect(game).toHaveAttribute('data-frequency', String(2 + seed));
    await expect(game).toHaveAttribute('data-tile-count', String(10 * (2 + seed) ** 2 + 2));
  }
  for (const viewport of [{ width: 540, height: 900 }, { width: 1440, height: 700 }]) {
    await page.setViewportSize(viewport);
    await expect(game).toHaveJSProperty('width', viewport.width);
    await expect(game).toHaveJSProperty('height', viewport.height);
    const rotation = await game.getAttribute('data-rotation');
    await page.mouse.move(viewport.width / 2, viewport.height / 2);
    await page.mouse.down();
    await page.mouse.move(viewport.width / 2 + 50, viewport.height / 2 + 20, { steps: 4 });
    await page.mouse.up();
    await expect(game).not.toHaveAttribute('data-rotation', rotation!);
    await page.mouse.click(viewport.width / 2, viewport.height / 2);
    await expect(game).toHaveAttribute('data-selected-tile', /\d+/);
    await expect(game).not.toHaveAttribute('data-error');
  }
  expect(errors).toEqual([]);
});
