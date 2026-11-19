import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("agentlens.locale", "en-US"));
});
const agents = async (page: import("@playwright/test").Page) =>
  page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Agents", exact: true })
    .click();

test("adopt a process tree, bind cache, restore its saved rule, and detach without stopping processes", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:1421");
  await agents(page);
  await page.getByRole("button", { name: "Adopt Agent", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Adopt Agent", exact: true });
  await dialog.getByRole("button", { name: /python3 PID 48120/ }).click();
  await dialog.getByLabel("Agent name", { exact: true }).fill("Research Lab");
  await dialog
    .getByRole("textbox", { name: "Directory path", exact: true })
    .fill("/Users/alex/.research-agent/cache");
  await dialog.getByRole("combobox", { name: "Directory usage" }).click();
  await page.getByRole("option", { name: "Cache", exact: true }).click();
  await dialog.getByRole("button", { name: "Add", exact: true }).click();
  await dialog.getByRole("button", { name: "Preview adoption", exact: true }).click();
  await expect(dialog.locator(".adoption-review-row")).toHaveCount(2);
  await expect(dialog.locator(".adoption-preview-totals")).toContainText("448.0 MB");
  await expect(dialog.locator(".adoption-bound-row")).toContainText("~/.research-agent/cache");
  await page.screenshot({
    path: "artifacts/screenshots/generic-adoption-preview.png",
  });
  await dialog.getByRole("button", { name: "Confirm adoption", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.locator(".agent-registry-row")).toContainText("Research Lab");
  await expect(page.locator(".agent-registry-row")).toContainText("2 processes");
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Processes", exact: true })
    .click();
  await page.getByRole("textbox", { name: "Search workspace" }).fill("Research Lab");
  await expect(page.locator(".process-table tbody > tr")).toHaveCount(2);
  await page.reload();
  await agents(page);
  await expect(page.locator(".agent-registry-row")).toContainText("Saved recognition rule");
  await expect(page.locator(".agent-registry-row")).toContainText("2 processes");
  await page.screenshot({
    path: "artifacts/screenshots/generic-agent-registry.png",
  });
  await page.getByRole("button", { name: "Detach Research Lab", exact: true }).click();
  await page.getByRole("button", { name: "Confirm detach", exact: true }).click();
  await expect(page.locator(".agent-registry-row")).toHaveCount(0);
  await page.getByRole("button", { name: "Adopt Agent", exact: true }).click();
  await expect(page.getByRole("button", { name: /python3 PID 48120/ })).toBeEnabled();
});

test("current-session adoption expires after reload while a manual rule can be saved with no running match", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:1421");
  await agents(page);
  await page.getByRole("button", { name: "Adopt Agent", exact: true }).click();
  let dialog = page.getByRole("dialog", { name: "Adopt Agent", exact: true });
  await dialog.getByRole("button", { name: /node PID 48130/ }).click();
  await dialog.getByLabel("Agent name", { exact: true }).fill("One Run");
  await dialog.getByRole("switch", { name: "Remember recognition rule" }).click();
  await dialog.getByRole("button", { name: "Preview adoption", exact: true }).click();
  await expect(dialog).toContainText("Current app session only");
  await dialog.getByRole("button", { name: "Confirm adoption", exact: true }).click();
  await expect(page.locator(".agent-registry-row")).toContainText("1 processes");
  await page.reload();
  await agents(page);
  await expect(page.locator(".agent-registry-row")).toContainText("0 processes");
  await page.getByRole("button", { name: "Adopt Agent", exact: true }).click();
  dialog = page.getByRole("dialog", { name: "Adopt Agent", exact: true });
  await dialog.getByRole("tab", { name: "Recognition rule", exact: true }).click();
  await dialog.getByLabel("Agent name", { exact: true }).fill("Future Agent");
  await dialog.getByLabel("Executable path", { exact: true }).fill("/usr/local/bin/future-agent");
  await dialog.getByRole("button", { name: "Preview adoption", exact: true }).click();
  await expect(dialog).toContainText("No running matches.");
  await dialog.getByRole("button", { name: "Confirm adoption", exact: true }).click();
  await expect(page.locator(".agent-registry-row")).toHaveCount(2);
});

test("directory overlaps are explained before confirmation", async ({ page }) => {
  await page.goto("http://127.0.0.1:1421");
  await agents(page);
  await page.getByRole("button", { name: "Adopt Agent", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Adopt Agent", exact: true });
  await dialog.getByRole("button", { name: /python3 PID 48120/ }).click();
  await dialog
    .getByRole("textbox", { name: "Directory path", exact: true })
    .fill("/Users/alex/.codex");
  await dialog.getByRole("button", { name: "Add", exact: true }).click();
  await dialog.getByRole("button", { name: "Preview adoption", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("overlaps");
  await expect(dialog.getByRole("button", { name: "Confirm adoption", exact: true })).toHaveCount(
    0,
  );
});
