import { expect, test } from "@playwright/test";

test("all 54 processes are accounted for, categories are disjoint and process recheck remains available", async ({
  page,
}) => {
  await page.addInitScript(() => localStorage.setItem("agentlens.locale", "en-US"));
  await page.route("**/src/lib/client.ts*", (route) => {
    if (new URL(route.request().url()).searchParams.has("actual")) return route.continue();
    return route.fulfill({
      contentType: "application/javascript",
      body: `
        export * from "/src/lib/client.ts?actual";
        import { demoSnapshot } from "/src/lib/demo.ts";
        const state = demoSnapshot();
        const categories = [
          ...Array(5).fill("active"),
          ...Array(45).fill("keep"),
          ...Array(2).fill("observing"),
          "review",
          "protected",
        ];
        state.processes = categories.map((category, i) => ({
          ...state.processes[0],
          pid: 1000 + i,
          name: "fixture-" + i,
          assessment: {
            category,
            reason: category === "keep" ? "attached_task" : "insufficient_evidence",
            idleSeconds: 0,
            orphanSeconds: 0,
          },
        }));
        window.processRechecks = 0;
        export async function snapshot() {
          return state;
        }
        export async function subscribe() {
          return () => {};
        }
        export async function refreshProcesses() {
          window.processRechecks += 1;
          return state;
        }
      `,
    });
  });
  await page.goto("http://127.0.0.1:1421");
  await page.locator(".sidebar").getByRole("button", { name: "Processes", exact: true }).click();
  await expect(page.locator(".process-triage")).toHaveCount(0);
  const tabs = page.getByRole("tablist", { name: "Process categories" });
  const counts = await tabs.locator(".tab-count").allTextContents();
  expect(counts.map(Number)).toEqual([54, 0, 5, 45, 2, 1, 1]);
  expect(counts.slice(1).reduce((total, n) => total + Number(n), 0)).toBe(54);
  for (const [label, count] of [
    ["Still working", 5],
    ["Idle / retained", 45],
    ["Still observing", 2],
    ["Needs further review", 1],
    ["Excluded", 1],
    ["Suggested cleanup", 0],
  ] as const) {
    await tabs.getByRole("tab", { name: `${label} ${count}`, exact: true }).click();
    await expect(page.locator(".process-table tbody > tr")).toHaveCount(count);
    if (count)
      await expect(page.locator(".process-table tbody > tr .badge").first()).toHaveText(label);
    if (label === "Excluded") {
      await expect(page.getByRole("button", { name: "Stop", exact: true })).toBeDisabled();
      await expect(
        page.getByRole("checkbox", { name: "Select all visible processes" }),
      ).toBeDisabled();
    }
    await expect(
      page.getByRole("button", { name: "Recheck processes", exact: true }),
    ).toBeVisible();
  }
  await page.getByRole("button", { name: "Recheck processes", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).processRechecks)).toBe(1);
  await expect(
    page.getByRole("button", { name: "Review suggested batch (0)", exact: true }),
  ).toBeDisabled();
  await expect(page.getByRole("status")).toContainText(["Process analysis updated"]);
});

test("header resources open only on click and remain interactive until dismissed", async ({
  page,
}) => {
  await page.addInitScript(() => localStorage.setItem("agentlens.locale", "en-US"));
  await page.goto("http://127.0.0.1:1421");
  const trigger = page.getByRole("button", { name: "Menu bar preview", exact: true });
  const panel = page.getByRole("dialog", { name: "Resource quick look" });
  await trigger.hover();
  await expect(panel).toHaveCount(0);
  await trigger.click();
  await expect(panel).toBeVisible();
  await panel.hover();
  await page.getByRole("heading", { name: "Overview", exact: true }).hover();
  await expect(panel).toBeVisible();
  await trigger.click();
  await expect(panel).toHaveCount(0);
  await trigger.click();
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await trigger.click();
  await page.getByRole("heading", { name: "Overview", exact: true }).click();
  await expect(panel).toHaveCount(0);
});
