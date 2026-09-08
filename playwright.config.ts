import { defineConfig } from '@playwright/test';

// Keep GPU backends isolated: a forced ANGLE SwiftShader backend disables
// Chromium's Windows WebGPU adapter discovery. Both projects use the Chromium
// revision bundled with the exact Playwright version in package-lock.json.
const webgpuArgs = ['--enable-unsafe-webgpu', '--use-webgpu-adapter=swiftshader'];
if (process.platform === 'linux') {
  // Linux software rendering needs Vulkan libraries and an Xvfb display in CI.
  webgpuArgs.push('--enable-features=Vulkan', '--use-angle=vulkan',
    '--use-vulkan=swiftshader', '--disable-vulkan-surface');
}

export default defineConfig({
  testDir: './tests/browser',
  timeout: 60000,
  expect: { timeout: 30000 },
  retries: 0,
  updateSnapshots: 'none',
  workers: 1,
  reporter: [['list'], ['html', { open: 'never' }]],
  snapshotPathTemplate: 'tests/baselines/{arg}{ext}',
  use: {
    channel: 'chromium',
    headless: process.env.CLAIMLANDS_HEADED !== '1',
    baseURL: process.env.PREVIEW_URL || 'http://127.0.0.1:4173',
    viewport: { width: 1280, height: 800 },
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: process.env.PREVIEW_URL ? undefined : {
    command: 'python -m http.server 4173 --bind 127.0.0.1 --directory dist',
    port: 4173, reuseExistingServer: !process.env.CI,
  },
  projects: [
    {
      name: 'webgl',
      metadata: { backend: 'webgl' },
      testMatch: ['game.spec.ts', 'visual.spec.ts'],
      use: {
        browserName: 'chromium',
        launchOptions: { args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] },
      },
    },
    {
      name: 'webgpu',
      metadata: { backend: 'webgpu' },
      testMatch: ['game.spec.ts', 'recovery.spec.ts'],
      use: {
        browserName: 'chromium',
        launchOptions: { args: webgpuArgs },
      },
    },
  ],
});
