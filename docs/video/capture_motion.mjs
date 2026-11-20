// Record real interactions in the browser preview. All data and operations are examples.
import { chromium } from "@playwright/test";
import { mkdir, rename } from "node:fs/promises";
const dir = new URL("./assets/motion/", import.meta.url).pathname;
await mkdir(dir, { recursive: true });
const browser = await chromium.launch({ channel: "chrome" });
async function clip(name, prepare, perform) {
  if (process.env.CLIP && process.env.CLIP !== name) return;
  const context = await browser.newContext({
    viewport: { width: 1440, height: 960 },
    recordVideo: { dir, size: { width: 1440, height: 960 } },
  });
  const page = await context.newPage();
  await page.addInitScript(() => {
    localStorage.setItem("agentlens.locale", "zh-CN");
    localStorage.setItem("agentlens.theme", "dark");
  });
  await page.goto("http://127.0.0.1:1421");
  await page.locator(".metric-card").first().waitFor();
  const nav = async (label) =>
    page
      .locator("nav button")
      .filter({ hasText: new RegExp("^" + label) })
      .first()
      .click();
  await prepare(page, nav);
  await page.waitForTimeout(400);
  const start = Date.now();
  await perform(page, nav);
  await page.waitForTimeout(Math.max(0, 4800 - (Date.now() - start)));
  await page.screenshot({ path: new URL(`./assets/${name}.png`, import.meta.url).pathname });
  const video = page.video();
  await context.close();
  await rename(await video.path(), `${dir}${name}.webm`);
}
try {
  await clip(
    "overview",
    async () => {},
    async (p) => {
      await p.getByRole("button", { name: "菜单栏速览预览", exact: true }).click();
      await p.waitForTimeout(1800);
      await p.keyboard.press("Escape");
    },
  );
  await clip(
    "processes",
    async (p, nav) => {
      await nav("进程");
      await p.locator(".process-table").scrollIntoViewIfNeeded();
    },
    async (p) => {
      await p.getByRole("tab", { name: /空闲保留/ }).click();
      await p.waitForTimeout(1000);
      await p.getByRole("tab", { name: /建议清理/ }).click();
      await p.waitForTimeout(1000);
      await p.getByRole("tab", { name: /全部进程/ }).click();
    },
  );
  await clip(
    "adoption",
    async (p, nav) => {
      await nav("Agent 管理");
      await p.getByRole("button", { name: "接管 Agent", exact: true }).click();
      await p.getByRole("tab", { name: "自定义识别规则", exact: true }).click();
    },
    async (p) => {
      await p.getByLabel("Agent 名称", { exact: true }).fill("桌面助手");
      await p
        .getByLabel("可执行文件路径", { exact: true })
        .fill("/Applications/ExampleAgent.app/Contents/MacOS/example-agent");
      await p.getByRole("button", { name: "预览接管范围", exact: true }).click();
    },
  );
  await clip(
    "cleanup",
    async (p, nav) => {
      await nav("空间清理");
    },
    async (p) => {
      await p.getByRole("button", { name: "全选", exact: true }).click();
      await p.waitForTimeout(900);
      await p.getByRole("button", { name: "检查已选文件", exact: true }).click();
    },
  );
} finally {
  await browser.close();
}
