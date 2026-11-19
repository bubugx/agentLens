// Capture actual browser preview screens. Start npm run dev first.
import { chromium } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import { mkdir } from 'node:fs/promises';
const dir = fileURLToPath(new URL('./assets/', import.meta.url));
await mkdir(dir, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 960 }, deviceScaleFactor: 1.5 });
  await page.addInitScript(() => {
    localStorage.setItem('agentlens.locale', 'zh-CN');
    localStorage.setItem('agentlens.theme', 'dark');
  });
  await page.goto('http://127.0.0.1:1421');
  await page.waitForSelector('.metric-card');
  await page.screenshot({ path: `${dir}overview.png` });
  for (const [label, file] of [['存储', 'storage'], ['进程', 'processes'], ['空间清理', 'cleanup'], ['活动', 'activity']]) {
    await page.locator('nav button').filter({ hasText: new RegExp('^' + label) }).first().click();
    await page.waitForTimeout(200);
    await page.screenshot({ path: `${dir}${file}.png` });
    if (file === 'processes') {
      await page.locator('.process-expand').nth(3).click();
      await page.screenshot({ path: `${dir}process-detail.png` });
    }
  }
} finally {
  await browser.close();
}
