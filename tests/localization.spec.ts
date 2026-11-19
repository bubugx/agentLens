import { test, expect } from "@playwright/test";
test("Chinese is the default; language and theme persist across reloads", async ({ page }) => {
  await page.goto("http://127.0.0.1:1421");
  await expect(page.getByRole("heading", { name: "总览", exact: true })).toBeVisible();
  await expect(
    page
      .getByRole("navigation", { name: "主导航" })
      .getByRole("button", { name: "存储空间", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "artifacts/screenshots/overview-zh.png",
    animations: "disabled",
  });
  await page.getByRole("button", { name: "设置", exact: true }).first().click();
  await expect(page.getByRole("combobox", { name: "采样频率" })).toHaveText("每 5 秒");
  await page.getByRole("combobox", { name: "显示语言" }).click();
  await expect(page.locator(".ui-select-content")).toBeVisible();
  await page.getByRole("option", { name: "English", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Settings", exact: true })).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Resource sampling interval" })).toHaveText(
    "Every 5 seconds",
  );
  await page.getByRole("button", { name: "Dark", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.getByRole("button", { name: "Dark", exact: true })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.screenshot({
    path: "artifacts/screenshots/settings-dark.png",
    animations: "disabled",
  });
  await page.reload();
  await expect(page.getByRole("heading", { name: "Overview", exact: true })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.getByRole("button", { name: "Settings", exact: true }).first().click();
  await page.getByRole("combobox", { name: "Display language" }).click();
  await page.getByRole("option", { name: "简体中文", exact: true }).click();
  await expect(page.getByRole("combobox", { name: "采样频率" })).toHaveText("每 5 秒");
  await page.getByRole("button", { name: "自定义主题", exact: true }).click();
  await expect(page.locator("select")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "自定义主题", exact: true })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(page.getByLabel("强调色")).toBeVisible();
  await page.getByLabel("强调色").fill("#7160bd");
  await page.getByRole("button", { name: "应用颜色", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "custom");
  await page.screenshot({
    path: "artifacts/screenshots/settings-zh.png",
    animations: "disabled",
  });
});
test("custom selects support keyboard control; resource quick look includes changes", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:1421");
  await page.getByRole("combobox", { name: "趋势指标" }).focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByRole("listbox")).toBeVisible();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("listbox")).toHaveCount(0);
  await page.getByRole("button", { name: "菜单栏速览预览" }).click();
  await expect(page.locator(".tray-popover")).toBeVisible();
  await expect(
    page.locator(".tray-popover").getByText("Agent 内存", { exact: true }),
  ).toBeVisible();
  await expect(page.locator(".tray-popover").getByText("Agent CPU", { exact: true })).toBeVisible();
  await expect(page.locator(".resource-change")).toHaveCount(2);
  await page.screenshot({
    path: "artifacts/screenshots/quicklook-zh.png",
    animations: "disabled",
  });
  await page
    .locator(".tray-popover")
    .getByRole("button", { name: /有文件可供清理/ })
    .click();
  await expect(page.getByRole("heading", { name: "空间清理", exact: true })).toBeVisible();
  await page
    .getByRole("navigation", { name: "主导航" })
    .getByRole("button", { name: "存储空间", exact: true })
    .click();
  await page.getByRole("combobox", { name: "筛选 Agent" }).click();
  await page.getByRole("option", { name: "Codex", exact: true }).click();
  await expect(page.locator(".file-row")).toHaveCount(3);
  await page.getByRole("combobox", { name: "筛选 Agent" }).click();
  await expect(page.getByRole("listbox")).toBeVisible();
  await page.screenshot({
    path: "artifacts/screenshots/storage-zh.png",
    animations: "disabled",
  });
});
