import { test, expect } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("agentlens.locale", "en-US"));
});
test("cleanup actions stay grouped at the right and distribution drills into exact files", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:1421");
  await page
    .getByRole("navigation", { name: "Utilities" })
    .getByRole("button", { name: /Cleanup/ })
    .click();
  const actions = page.locator(".page-heading-actions");
  const rules = actions.getByRole("button", { name: "Adjust cleanup rules" }),
    scan = actions.getByRole("button", { name: "Recheck files" });
  for (const width of [1440, 1080]) {
    await page.setViewportSize({ width, height: 960 });
    const a = await rules.boundingBox(),
      b = await scan.boundingBox(),
      heading = await page.locator(".page-heading").boundingBox();
    expect(Math.abs(a!.y - b!.y)).toBeLessThan(3);
    expect(b!.x - a!.x - a!.width).toBeLessThan(16);
    expect(Math.abs(b!.x + b!.width - heading!.x - heading!.width)).toBeLessThan(3);
  }
  await page
    .getByRole("button", {
      name: "View Programs / data protected distribution",
    })
    .click();
  const modal = page.getByRole("dialog");
  await expect(modal).toBeVisible();
  await expect(modal.locator(".inventory-group")).toHaveCount(2);
  await expect(modal.locator(".file-row")).toHaveCount(2);
  await modal.getByRole("button", { name: "Inspect directory Cursor storage" }).click();
  await expect(modal.locator(".file-row")).toHaveCount(1);
  await expect(
    modal.getByText("workspaceStorage/agentlens/state.vscdb", { exact: true }),
  ).toBeVisible();
  await modal.getByRole("button", { name: "All directories" }).click();
  await modal.getByRole("textbox", { name: "Search inventory files" }).fill("settings.json");
  await expect(modal.locator(".file-row")).toHaveCount(1);
  await expect(modal.getByText("settings.json", { exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(modal).toHaveCount(0);
});
test("first inventory shows real scan stages, supports pause and keeps pages usable", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:1421");
  await page.getByRole("button", { name: "Settings", exact: false }).click();
  await page.getByRole("button", { name: "Scan user space" }).click();
  await expect(page.getByText("Building your first inventory", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(page.getByText("Inventory scan paused", { exact: true })).toBeVisible();
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Overview", exact: true })
    .click();
  await page.getByRole("button", { name: "Resume", exact: true }).click();
  await expect(
    page.getByText("Finding your existing agent footprint", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("progressbar", { name: "File inventory progress" })).toBeVisible();
  await expect(page.getByText("Initial inventory ready", { exact: true })).toBeVisible({
    timeout: 10000,
  });
  await expect(page.locator(".metrics-grid")).toBeVisible();
  await expect(
    page.getByText(
      "Existing agent files become your storage baseline. Memory and CPU history starts with live observations.",
      { exact: true },
    ),
  ).toBeVisible();
});

test("folder access waits keep elapsed time live and report a partial inventory", async ({
  page,
}) => {
  await page.clock.install();
  await page.route("**/src/lib/client.ts*", (route) => {
    if (new URL(route.request().url()).searchParams.has("actual")) return route.continue();
    return route.fulfill({
      contentType: "application/javascript",
      body: `
        export * from "/src/lib/client.ts?actual";
        import { demoSnapshot } from "/src/lib/demo.ts";
        const state = demoSnapshot();
        Object.assign(state, { scanning: true, scannedAt: 0, scanComplete: false });
        Object.assign(state.scanProgress, {
          phase: "discovery",
          initial: true,
          elapsedMs: 0,
          directories: 0,
          files: 0,
          rootsDone: 0,
          currentPath: "/Users/alex/Desktop",
        });
        let receive;
        const updates = setInterval(() => receive?.({ ...state }), 2000);
        export async function snapshot() {
          return state;
        }
        export async function subscribe(callback) {
          receive = callback;
          return () => {};
        }
        window.finishTestScan = () => {
          clearInterval(updates);
          receive({
            ...state,
            scanning: false,
            scanProgress: {
              ...state.scanProgress,
              phase: "complete",
              elapsedMs: 9000,
              errors: 1,
            },
          });
        };
      `,
    });
  });
  await page.goto("http://127.0.0.1:1421");
  const status = page.locator(".scan-status");
  await expect(status.getByText("0s elapsed", { exact: true })).toBeVisible();
  await page.clock.fastForward(9000);
  await expect(status.getByText("9s elapsed", { exact: true })).toBeVisible();
  await expect(status.getByText("0 files indexed", { exact: true })).toBeVisible();
  await expect(status.getByRole("status")).toContainText("If your system asks for folder access");
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Processes", exact: true })
    .click();
  await expect(page.locator(".process-table")).toBeVisible();
  await page.evaluate("window.finishTestScan()");
  await expect(status.getByText("Inventory ready · some folders unavailable")).toBeVisible();
  await page.clock.fastForward(5000);
  await expect(status.getByText("9s elapsed", { exact: true })).toBeVisible();
  await expect(status.getByRole("status")).toHaveCount(0);
});
