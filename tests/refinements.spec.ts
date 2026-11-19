import { test, expect, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem("agentlens.locale", "en-US"),
  );
});
async function liveFixture(page: Page) {
  await page.route("**/src/lib/client.ts*", (route) => {
    if (new URL(route.request().url()).searchParams.has("actual"))
      return route.continue();
    return route.fulfill({
      contentType: "application/javascript",
      body: `
      export * from '/src/lib/client.ts?actual';
      import { demoSnapshot } from '/src/lib/demo.ts';
      let state = demoSnapshot(), receive;
      state.roots = Array.from({length:25}, (_,i) => ({id:'root-'+i,path:'/Users/alex/Developer/project-'+i,label:'Project '+i,agent:'codex',custom:i>=5,source:i>=5?'session':'global'}));
      state.activity = Array.from({length:35}, (_,i) => ({id:'event-'+i,time:state.timestamp-i*120,kind:'growth',agent:'codex',title:'Record '+i,detail:'Observed net change in monitored directories',bytes:100+i}));
      export async function snapshot(){return state;}
      export async function subscribe(callback){receive=callback;return ()=>{};}
      window.publishBackground = phase => {
        state = {...state,scanning:phase!=='complete',scanProgress:{...state.scanProgress,initial:false,background:true,phase}};
        receive(state);
      };
      window.publishObservation = () => {
        state = {...state,activity:[{id:'newest',time:state.timestamp+120,kind:'growth',agent:'codex',title:'New observation',detail:'new',bytes:200},...state.activity]};
        receive(state);
      };
    `,
    });
  });
}

test("cleanup policy stages changes, retains session options, and removes obsolete sidebar controls", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:1421");
  await expect(page.locator(".sidebar-bottom")).toHaveCount(0);
  await page.getByRole("button", { name: "About AgentLens" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("switch", { name: "Glass appearance" }),
  ).toHaveCount(0);
  const rules = page.locator(".cleanup-rules");
  await rules
    .getByRole("switch", { name: "Session helper caches", exact: true })
    .click();
  await rules
    .getByRole("switch", {
      name: "Include old session histories for manual review",
      exact: true,
    })
    .click();
  await rules
    .getByRole("combobox", { name: "Session scope", exact: true })
    .click();
  await page
    .getByRole("option", { name: "Archived sessions only", exact: true })
    .click();
  await rules
    .getByRole("combobox", { name: "Newest sessions to retain" })
    .click();
  await page
    .getByRole("option", {
      name: "20 session files per directory",
      exact: true,
    })
    .click();
  await rules
    .getByRole("textbox", { name: "Excluded cleanup path" })
    .fill("/Users/alex/Developer/keep");
  await rules
    .getByRole("button", { name: "Exclude path", exact: true })
    .click();
  await rules.getByRole("button", { name: "Claude Code", exact: true }).click();
  await expect(rules.getByRole("status")).toHaveText("Unapplied changes");
  await rules.getByRole("button", { name: "Apply cleanup rules" }).click();
  await expect(rules.getByRole("status")).toHaveText("Rules are up to date");
  await page
    .getByRole("navigation", { name: "Main navigation" })
    .getByRole("button", { name: "Overview", exact: true })
    .click();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(
    rules.getByRole("combobox", { name: "Session scope", exact: true }),
  ).toHaveText("Archived sessions only");
  await expect(
    rules.getByRole("combobox", { name: "Newest sessions to retain" }),
  ).toHaveText("20 session files per directory");
  await expect(
    rules.getByRole("switch", { name: "Session helper caches", exact: true }),
  ).toBeChecked();
  await expect(
    rules.getByRole("button", { name: "Claude Code", exact: true }),
  ).toHaveAttribute("aria-pressed", "false");
  await expect(
    rules.getByText("/Users/alex/Developer/keep", { exact: true }),
  ).toBeVisible();
});

test("directory pages filter correctly and background scan stages do not move the page", async ({
  page,
}) => {
  await liveFixture(page);
  await page.goto("http://127.0.0.1:1421");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const directories = page.locator(".monitored-directories");
  const pager = directories.getByRole("navigation", {
    name: "Directory pagination",
  });
  await expect(directories.locator(".directory-row")).toHaveCount(10);
  await pager.getByRole("button", { name: "Next page" }).click();
  await expect(
    directories.getByText("Project 10", { exact: true }),
  ).toBeVisible();
  await expect(directories.getByText("Project 0", { exact: true })).toHaveCount(
    0,
  );
  await directories
    .getByRole("tab", { name: "Global agent storage", exact: true })
    .click();
  await expect(directories.locator(".directory-row")).toHaveCount(5);
  await expect(pager.getByText("Page 1 of 1")).toBeVisible();
  await directories
    .getByRole("tab", { name: "Project directories", exact: true })
    .click();
  await directories
    .getByRole("textbox", { name: "Search monitored directories" })
    .fill("project-24");
  await expect(directories.locator(".directory-row")).toHaveCount(1);
  const before = await directories.boundingBox();
  const scrollBefore = await page
    .locator(".main-scroll")
    .evaluate((el) => el.scrollTop);
  for (const phase of ["indexing", "saving", "complete"]) {
    await page.evaluate((p) => (window as any).publishBackground(p), phase);
    await expect(page.locator(".scan-status")).toHaveCount(0);
    const after = await directories.boundingBox();
    expect(Math.abs(before!.y - after!.y)).toBeLessThan(1);
    expect(
      await page.locator(".main-scroll").evaluate((el) => el.scrollTop),
    ).toBe(scrollBefore);
  }
});

test("recent observations paginate and stay stable when new observations arrive", async ({
  page,
}) => {
  await liveFixture(page);
  await page.goto("http://127.0.0.1:1421");
  await page
    .getByRole("navigation", { name: "Utilities" })
    .getByRole("button", { name: "Activity", exact: true })
    .click();
  const ledger = page.locator(".activity-ledger");
  const pager = ledger.getByRole("navigation", { name: "Activity pagination" });
  await expect(ledger.locator(".ledger-group")).toHaveCount(10);
  await pager.getByRole("button", { name: "Next page" }).click();
  await expect(ledger.getByText("Record 10", { exact: true })).toBeVisible();
  const entries = await ledger.locator(".ledger-copy strong").allTextContents();
  await page.evaluate(() => (window as any).publishObservation());
  await expect(
    ledger.getByRole("button", { name: "Load 1 new observations" }),
  ).toBeVisible();
  expect(await ledger.locator(".ledger-copy strong").allTextContents()).toEqual(
    entries,
  );
  await ledger.getByRole("button", { name: "Load 1 new observations" }).click();
  await expect(
    ledger.getByText("New observation", { exact: true }),
  ).toBeVisible();
  await expect(pager.getByText("Page 1 of 4")).toBeVisible();
  await pager.getByRole("combobox", { name: "Entries per page" }).click();
  await page.getByRole("option", { name: "20 per page" }).click();
  await expect(ledger.locator(".ledger-group")).toHaveCount(20);
  await pager.getByRole("button", { name: "Next page" }).click();
  await expect(ledger.locator(".ledger-group")).toHaveCount(16);
  await expect(pager.getByRole("button", { name: "Next page" })).toBeDisabled();
});
