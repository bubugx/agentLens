import { test, expect, type Page } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("agentlens.locale", "en-US"));
});
async function activity(page: Page) {
  await page.goto("http://127.0.0.1:1421");
  await page
    .getByRole("navigation", { name: "Utilities" })
    .getByRole("button", { name: "Activity", exact: true })
    .click();
}
test("activity tabs show category-specific data and a storage curve only in the storage view", async ({
  page,
}) => {
  await activity(page);
  const tabs = page.getByRole("tablist", { name: "Activity categories" }),
    ledger = page.locator(".activity-ledger");
  await expect(page.locator(".activity-trend-panel")).toHaveCount(0);
  await expect(ledger.locator(".ledger-group")).toHaveCount(6);
  await tabs.getByRole("tab", { name: "Storage changes", exact: true }).click();
  await expect(page.getByRole("img", { name: "Storage trend chart" })).toBeVisible();
  await expect(page.locator(".activity-trend-line")).toHaveAttribute("d", /^M.+L/);
  await expect(ledger.locator(".ledger-group")).toHaveCount(3);
  await expect(ledger).not.toContainText("Old temporary files moved to Trash");
  await tabs.getByRole("tab", { name: "Cleanups", exact: true }).click();
  await expect(page.locator(".activity-trend-panel")).toHaveCount(0);
  await expect(page.locator(".activity-summary")).toContainText("Moved file size");
  await expect(ledger.locator(".ledger-group")).toHaveCount(1);
  await tabs.getByRole("tab", { name: "Processes", exact: true }).click();
  await expect(page.locator(".process-analytics")).toBeVisible();
  await expect(page.locator(".activity-summary")).toContainText("Attention signals");
  await expect(page.locator(".activity-trend-panel")).toHaveCount(0);
  await expect(ledger.locator(".ledger-group")).toHaveCount(1);
  await expect(ledger.locator(".ledger-process-reading")).toHaveCount(1);
  await expect(ledger.locator(".ledger-process-reading")).toContainText("0.1%");
  await expect(ledger.getByRole("button", { name: "View process readings" })).toHaveCount(0);
  await ledger.getByRole("button", { name: "View resource trend" }).click();
  await expect(page.locator(".resource-process-context")).toContainText("Project attribution");
  await tabs.getByRole("tab", { name: "Project discovery", exact: true }).click();
  await expect(page.locator(".process-analytics")).toHaveCount(0);
  await expect(ledger).toContainText("Projects discovered automatically");
  await ledger.getByRole("button", { name: "View project directories" }).click();
  await expect(ledger.locator(".ledger-project-path")).toHaveCount(2);
  await tabs.getByRole("tab", { name: "All activity", exact: true }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(tabs.getByRole("tab", { name: "Storage changes", exact: true })).toHaveAttribute(
    "aria-selected",
    "true",
  );
});
test("project rankings sort by resource use and drill into each process CPU and memory history", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:1421");
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Processes", exact: true })
    .click();
  const analysis = page.locator(".process-analytics");
  const strip = await page.locator(".resource-strip").boundingBox();
  const analysisBounds = await analysis.boundingBox();
  expect(analysisBounds!.y - strip!.y - strip!.height).toBeGreaterThanOrEqual(19);
  await expect(analysis.getByRole("img", { name: "cpu resource trend" })).toBeVisible();
  await expect(analysis.getByRole("img", { name: "memory resource trend" })).toBeVisible();
  await expect(analysis.locator(".resource-curve .activity-trend-line").first()).toHaveAttribute(
    "d",
    /^M.+L/,
  );
  const memoryValues = await analysis
    .locator(".resource-ranking-row > span:nth-child(3) strong")
    .allTextContents();
  expect(memoryValues.length).toBeGreaterThan(1);
  await analysis.getByRole("combobox", { name: "Sort resource usage" }).click();
  await page.getByRole("option", { name: "CPU · highest first" }).click();
  const cpu = await analysis
    .locator(".resource-ranking-row > span:nth-child(2) strong")
    .allTextContents();
  expect(cpu.map(parseFloat)).toEqual(cpu.map(parseFloat).sort((a, b) => b - a));
  await analysis.locator(".resource-ranking-row").first().click();
  await expect(analysis).toContainText("Processes in this project");
  await analysis.locator(".resource-ranking-row").first().click();
  await expect(analysis.locator(".resource-process-context")).toContainText("Project attribution");
  await expect(analysis.getByRole("img", { name: "memory resource trend" })).toBeVisible();
  await analysis.getByRole("button", { name: "Back", exact: true }).click();
  await expect(analysis.locator(".resource-ranking-row").first()).toBeVisible();
});
test("storage ranking is descending with small agents visible, cards are spaced and warnings do not move content", async ({
  page,
}) => {
  await page.route("**/src/lib/client.ts*", (route) => {
    if (new URL(route.request().url()).searchParams.has("actual")) return route.continue();
    return route.fulfill({
      contentType: "application/javascript",
      body: `
        export * from "/src/lib/client.ts?actual";
        import { demoSnapshot } from "/src/lib/demo.ts";
        let state = demoSnapshot(),
          receive;
        export async function snapshot() {
          return state;
        }
        export async function subscribe(cb) {
          receive = cb;
          return () => {};
        }
        window.issue = () => {
          state = { ...state, warnings: ["Fixture folder unavailable"] };
          receive(state);
        };
      `,
    });
  });
  await page.goto("http://127.0.0.1:1421");
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Storage", exact: true })
    .click();
  const summary = page.locator(".storage-summary"),
    assessment = page.locator(".cleanup-diagnostics");
  const a = await summary.boundingBox(),
    b = await assessment.boundingBox();
  expect(b!.y - a!.y - a!.height).toBeGreaterThanOrEqual(19);
  const sizes = await page.locator(".agent-storage-row strong").allTextContents();
  expect(sizes[0]).toBe("12.8 GB");
  await expect(page.locator(".agent-storage-row").first()).toContainText("Claude Code");
  const before = await summary.boundingBox();
  await page.evaluate(() => (window as any).issue());
  await expect(page.locator(".message-notification.warning")).toContainText(
    "Fixture folder unavailable",
  );
  expect((await summary.boundingBox())!.y).toBe(before!.y);
  await expect(page.locator(".warning-banner")).toHaveCount(0);
  await page.getByRole("button", { name: "Menu bar preview" }).click();
  const close = page.locator(".popover-close");
  await close.evaluate((el) => (el as HTMLElement).focus());
  expect(await close.evaluate((el) => getComputedStyle(el).outlineStyle)).toBe("none");
});
