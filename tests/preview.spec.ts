import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("agentlens.locale", "en-US");
  });
});

test("workspace navigation, filtering and resource trends", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("http://127.0.0.1:1421");
  await expect(page.getByRole("heading", { name: "Overview", exact: true })).toBeVisible();
  await expect(page.getByText("Preview · sample data")).toBeVisible();
  await page.getByRole("tab", { name: "1H", exact: true }).click();
  await page.getByRole("combobox", { name: "Chart metric" }).click();
  await page.getByRole("option", { name: "Agent memory", exact: true }).click();
  await expect(page.getByLabel("memory history chart")).toBeVisible();
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Storage", exact: true })
    .click();
  await page.getByLabel("Search workspace").fill("codex");
  await expect(page.locator(".file-row")).toHaveCount(3);
  await page.getByRole("button", { name: "Clear search" }).click();
  await page.locator(".filter-tabs").getByRole("tab", { name: "Sessions", exact: true }).click();
  await expect(page.locator(".file-row")).toHaveCount(2);
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Sessions", exact: true })
    .click();
  await expect(page.getByRole("heading", { name: "Sessions", exact: true })).toBeVisible();
  await expect(page.locator(".sessions-table tbody tr")).toHaveCount(14);
  expect(errors).toEqual([]);
});

test("cleanup is reviewable, cancellable and changes sample data only", async ({ page }) => {
  await page.goto("http://127.0.0.1:1421");
  await page
    .getByRole("navigation", { name: "Utilities" })
    .getByRole("button", { name: /Cleanup/ })
    .click();
  await expect(page.getByRole("button", { name: "Review selected files" })).toBeDisabled();
  await page.getByRole("button", { name: "Select all", exact: true }).click();
  await page.getByRole("button", { name: "Review selected files" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.getByText("This is a preview. No real files will be moved.")).toBeVisible();
  await page.getByRole("button", { name: "Keep files" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "Review selected files" }).click();
  await page.getByRole("button", { name: "Run preview cleanup" }).click();
  await expect(page.getByText("6 sample files cleaned up")).toBeVisible();
  await page.getByRole("button", { name: "All done" }).click();
  await expect(page.getByRole("heading", { name: "Nothing to clean up right now" })).toBeVisible();
});

test("leftover processes require confirmation and preferences work", async ({ page }) => {
  await page.goto("http://127.0.0.1:1421");
  await page.getByRole("button", { name: "Review processes", exact: true }).click();
  await page.getByRole("tab", { name: /Suggested cleanup/ }).click();
  await expect(page.locator(".process-table tbody tr")).toHaveCount(1);
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Stop esbuild?" })).toBeVisible();
  await page.getByRole("button", { name: "Keep running" }).click();
  await expect(page.locator(".process-table tbody tr")).toHaveCount(1);
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await page.getByRole("button", { name: "Stop sample process" }).click();
  await expect(page.getByRole("heading", { name: "No suggested leftovers yet" })).toBeVisible();
  await page.getByRole("button", { name: /^Settings/ }).click();
  await expect(page.getByRole("switch", { name: "Glass appearance" })).toHaveCount(0);
  await page.getByRole("combobox", { name: "Resource sampling interval" }).click();
  await page.getByRole("option", { name: "Every 10 seconds" }).click();
  await expect(page.getByRole("combobox", { name: "Resource sampling interval" })).toHaveText(
    /Every 10 seconds/,
  );
  await page.getByRole("button", { name: "Add project", exact: true }).click();
  await page.getByRole("button", { name: "Browse", exact: true }).click();
  await page.getByRole("button", { name: "Add sample project" }).click();
  await expect(page.getByText("new-project", { exact: true })).toBeVisible();
});

test("desktop layouts have no horizontal overflow and dialogs trap focus", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.goto("http://127.0.0.1:1421");
  await page.screenshot({
    path: "artifacts/screenshots/overview.png",
    fullPage: true,
  });
  for (const name of ["Agents", "Storage", "Sessions", "Processes"]) {
    await page
      .getByRole("navigation", { name: "Main navigation" })
      .getByRole("button", { name: new RegExp(`^${name}`) })
      .click();
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
    ).toBeTruthy();
  }
  await page.setViewportSize({ width: 1080, height: 760 });
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Overview" })
    .click();
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
  ).toBeTruthy();
  await page.getByRole("button", { name: "View Claude Code" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Shift+Tab");
  await expect(page.getByRole("button", { name: "Explore storage" })).toBeFocused();
  const help = page.getByRole("button", { name: "Storage association" });
  await help.click();
  await expect(page.locator(".context-help-content")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".context-help-content")).toHaveCount(0);
  await expect(page.getByRole("dialog", { name: "Claude Code" })).toBeVisible();
  await expect(help).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
