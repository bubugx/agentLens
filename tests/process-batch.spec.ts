import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem("agentlens.locale", "en-US"),
  );
});

test("process multi-selection requires confirmation and stops exactly the selected sample identities", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:1421");
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Processes", exact: true })
    .click();
  const rows = page.locator(".process-table tbody > tr");
  const initialCount = await rows.count();
  const all = page.getByRole("checkbox", {
    name: "Select all visible processes",
  });
  const action = page.getByRole("button", {
    name: "Stop selected processes",
    exact: true,
  });
  await expect(action).toBeDisabled();
  await rows.nth(0).getByRole("checkbox").click();
  await rows.nth(1).getByRole("checkbox").click();
  await expect(all).toHaveAttribute("aria-checked", "mixed");
  await expect(page.locator(".process-selection-summary")).toContainText(
    "2 processes selected",
  );
  await action.click();
  let dialog = page.getByRole("dialog", {
    name: "Stop selected processes?",
    exact: true,
  });
  await expect(dialog.locator(".batch-stop-item")).toHaveCount(2);
  await dialog.getByRole("button", { name: "Keep running" }).click();
  await expect(rows).toHaveCount(initialCount);
  await action.click();
  dialog = page.getByRole("dialog", {
    name: "Stop selected processes?",
    exact: true,
  });
  await dialog
    .getByRole("button", { name: "Stop sample processes", exact: true })
    .click();
  const result = page.getByRole("dialog", {
    name: "Batch process results",
    exact: true,
  });
  await expect(result.getByRole("status")).toContainText(
    "2 sample processes stopped · 0 failed",
  );
  await expect(rows).toHaveCount(initialCount - 2);
  await result.getByRole("button", { name: "Done", exact: true }).click();
  await expect(action).toBeDisabled();
  await all.click();
  await expect(all).toHaveAttribute("aria-checked", "true");
  await page
    .getByRole("button", { name: "Clear selection", exact: true })
    .click();
  await expect(all).toHaveAttribute("aria-checked", "false");
});

test("selection follows the visible list and never transfers to a reused PID", async ({
  page,
}) => {
  await page.route("**/src/lib/client.ts*", (route) => {
    if (new URL(route.request().url()).searchParams.has("actual"))
      return route.continue();
    return route.fulfill({
      contentType: "application/javascript",
      body: `
      export * from '/src/lib/client.ts?actual';
      import { demoSnapshot } from '/src/lib/demo.ts';
      let state = demoSnapshot(), receive;
      export async function snapshot() { return state; }
      export async function subscribe(callback) { receive = callback; return () => {}; }
      window.reusePid = () => {
        state = {...state, processes: state.processes.map((p,i) => i ? p : {...p,startTime:p.startTime+1})};
        receive(state);
      };
    `,
    });
  });
  await page.goto("http://127.0.0.1:1421");
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Processes", exact: true })
    .click();
  const first = page
    .locator(".process-table tbody > tr")
    .first()
    .getByRole("checkbox");
  await first.click();
  await expect(first).toHaveAttribute("aria-checked", "true");
  await page.evaluate(() => (window as any).reusePid());
  await expect(first).toHaveAttribute("aria-checked", "false");
  await page
    .getByRole("checkbox", { name: "Select all visible processes" })
    .click();
  await page.getByRole("tab", { name: /Needs attention/ }).click();
  await expect(page.locator(".process-selection-summary")).toContainText(
    "1 processes selected",
  );
  await page.getByRole("tab", { name: "All processes", exact: true }).click();
  await expect(
    page.locator(".process-table tbody [role=checkbox][aria-checked=true]"),
  ).toHaveCount(1);
});
test("automatic suggestions group idle leftovers and stop the suggested batch after one confirmation", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:1421");
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Processes", exact: true })
    .click();
  const triage = page.locator(".process-triage");
  const action = triage.getByRole("button", {
    name: "Review suggested batch (1)",
    exact: true,
  });
  await expect(action).toBeEnabled();
  await action.click();
  let dialog = page.getByRole("dialog", {
    name: "Stop selected processes?",
    exact: true,
  });
  await expect(dialog.locator(".batch-stop-item")).toHaveCount(1);
  await expect(dialog.locator(".batch-stop-item")).toContainText("esbuild");
  await expect(dialog.locator(".batch-suggestion-reason")).toContainText(
    "idle for 10 minutes",
  );
  await dialog
    .getByRole("button", { name: "Keep running", exact: true })
    .click();
  await expect(page.locator(".process-table tbody > tr")).toHaveCount(9);
  await triage
    .getByRole("button", { name: "View suggestions", exact: true })
    .click();
  await expect(
    page.getByRole("tab", { name: "Suggested cleanup 1", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".process-table tbody > tr")).toHaveCount(1);
  await action.click();
  dialog = page.getByRole("dialog", {
    name: "Stop selected processes?",
    exact: true,
  });
  await dialog
    .getByRole("button", { name: "Stop sample processes", exact: true })
    .click();
  await expect(
    page
      .getByRole("dialog", { name: "Batch process results" })
      .getByRole("status"),
  ).toContainText("1 sample processes stopped · 0 failed");
  await page.getByRole("button", { name: "Done", exact: true }).click();
  await expect(action).toHaveCount(0);
  await expect(
    triage.getByRole("button", {
      name: "Review suggested batch (0)",
      exact: true,
    }),
  ).toBeDisabled();
});
