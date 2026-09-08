import { test, expect } from '@playwright/test';

test('deployed game restores its world after an actual WebGPU device loss', async ({ page }) => {
  await page.addInitScript(() => {
    type Device = { destroy(): void };
    type ProbeWindow = Window & {
      GPUAdapter: { prototype: { requestDevice(...args: unknown[]): Promise<Device> } };
      devicesForRecoveryTest: Device[];
    };
    const probe = window as unknown as ProbeWindow;
    probe.devicesForRecoveryTest = [];
    const original = probe.GPUAdapter.prototype.requestDevice;
    probe.GPUAdapter.prototype.requestDevice = async function (...args) {
      const device = await original.apply(this, args);
      probe.devicesForRecoveryTest.push(device);
      return device;
    };
  });
  await page.goto('/?backend=webgpu&frequency=8&seed=63352');
  const game = page.locator('#game');
  await expect(game).toHaveAttribute('data-ready', 'true');
  await expect(game).toHaveAttribute('data-backend', 'BrowserWebGpu');
  const fingerprint = await game.getAttribute('data-fingerprint');
  const rotation = await game.getAttribute('data-rotation');
  await page.evaluate(() => {
    const probe = window as unknown as { devicesForRecoveryTest: { destroy(): void }[] };
    if (probe.devicesForRecoveryTest.length !== 1) throw new Error('Expected one live device');
    probe.devicesForRecoveryTest[0].destroy();
  });
  await expect(game).toHaveAttribute('data-device-recovery', '1');
  await expect(game).toHaveAttribute('data-ready', 'true');
  await expect(game).not.toHaveAttribute('data-error');
  await expect(game).toHaveAttribute('data-fingerprint', fingerprint!);
  await expect(game).toHaveAttribute('data-rotation', rotation!);
  await page.mouse.click(640, 400);
  await expect(game).toHaveAttribute('data-selected-tile', /\d+/);
  await page.keyboard.press('n');
  await expect(game).toHaveAttribute('data-seed', '63353');
  await expect(game).not.toHaveAttribute('data-error');
});
