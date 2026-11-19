import { expect, test, type Page } from "@playwright/test";

async function firstScan(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem("agentlens.locale", "en-US");
    localStorage.setItem("agentlens.theme", "dark");
  });
  await page.route("**/src/lib/client.ts*", (route) => {
    if (new URL(route.request().url()).searchParams.has("actual"))
      return route.continue();
    return route.fulfill({
      contentType: "application/javascript",
      body: `
        export * from '/src/lib/client.ts?actual';
        import { demoSnapshot } from '/src/lib/demo.ts';
        let state = demoSnapshot(), receive, lastPhase = 'discovery';
        Object.assign(state, { scanning: true, scannedAt: 0, scanComplete: false });
        Object.assign(state.scanProgress, {
          initial: true, background: false, phase: 'discovery', elapsedMs: 0,
          files: 0, directories: 0, rootsDone: 0, rootsTotal: 0, bytes: 0,
        });
        export async function snapshot() { return state; }
        export async function subscribe(callback) { receive = callback; return () => {}; }
        function publish(phase) {
          if (['discovery', 'indexing', 'saving'].includes(phase)) lastPhase = phase;
          state = { ...state,
            scanning: !['paused','complete','error'].includes(phase),
            monitoring: phase !== 'paused',
            scannedAt: phase === 'complete' ? state.timestamp : 0,
            scanComplete: phase === 'complete',
            scanProgress: { ...state.scanProgress, phase,
              ...(phase === 'indexing' ? { files: 120, rootsDone: 2, rootsTotal: 4 } : {}),
            },
          };
          receive(state);
        }
        export async function monitoring(enabled) { publish(enabled ? lastPhase : 'paused'); }
        window.publishScan = publish;
      `,
    });
  });
  await page.goto("http://127.0.0.1:1421/");
  await expect(page.locator(".initial-scan")).toBeVisible();
}

test("first scan motion follows real stages, freezes on pause and releases the overview on completion", async ({
  page,
}) => {
  await firstScan(page);
  const scene = page.locator(".initial-scan");
  const progress = page.getByRole("progressbar", {
    name: "File inventory progress",
  });
  const currentStage = page.locator('.scan-stages [aria-current="step"]');
  await expect(currentStage).toContainText("Discover directories");
  await expect(progress).not.toHaveAttribute("aria-valuenow");

  const sweep = page.locator(".scan-lens-sweep");
  const initialTransform = await sweep.evaluate(
    (el) => getComputedStyle(el).transform,
  );
  await expect
    .poll(() => sweep.evaluate((el) => getComputedStyle(el).transform))
    .not.toBe(initialTransform);
  await expect(page.locator(".scan-status-stats")).toContainText(
    "0 files indexed",
  );

  await page.evaluate(() => (window as any).publishScan("indexing"));
  await expect(currentStage).toContainText("Build file index");
  await expect(progress).toHaveAttribute("aria-valuenow", "50");
  await expect(page.locator(".scan-status-stats")).toContainText(
    "120 files indexed",
  );

  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(scene).toHaveAttribute("aria-busy", "false");
  await expect(sweep).toHaveCSS("animation-play-state", "paused");
  await expect(currentStage).toContainText("Build file index");
  await expect(progress).toHaveCount(0);
  await page.getByRole("button", { name: "Resume", exact: true }).click();
  await expect(sweep).toHaveCSS("animation-play-state", "running");

  await page.evaluate(() => (window as any).publishScan("saving"));
  await expect(currentStage).toContainText("Prepare storage map");
  await expect(progress).not.toHaveAttribute("aria-valuenow");
  await page.evaluate(() => (window as any).publishScan("complete"));
  await expect(scene).toHaveCount(0);
  await expect(page.locator(".metrics-grid")).toBeVisible();
  await expect(
    page.getByText("Initial inventory ready", { exact: true }),
  ).toBeVisible();
});

test("reduced motion leaves scan stages readable and navigation available", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1080, height: 760 });
  await firstScan(page);
  const scene = page.locator(".initial-scan");
  await expect(page.locator(".scan-lens-sweep")).toHaveCSS(
    "animation-name",
    "none",
  );
  await expect(page.locator(".scan-signal").first()).toBeHidden();
  await expect(scene.getByRole("list", { name: "Scan stages" })).toBeVisible();
  const bounds = await page
    .locator(".main-scroll")
    .evaluate((el) => [el.scrollWidth, el.clientWidth]);
  expect(bounds[0]).toBeLessThanOrEqual(bounds[1]);
  await scene.getByRole("button", { name: "Processes", exact: true }).click();
  await expect(page.locator(".process-table")).toBeVisible();
  await expect(page.locator(".scan-status")).toBeVisible();
});
