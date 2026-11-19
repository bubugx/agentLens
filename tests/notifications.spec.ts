import { expect, test } from "@playwright/test";

test("floating messages preserve layout, pause while being used, and support actions and dismissal", async ({
  page,
}) => {
  await page.clock.install();
  await page.addInitScript(() => localStorage.setItem("agentlens.locale", "en-US"));
  let notificationModule = "";
  page.on("request", (request) => {
    if (new URL(request.url()).pathname === "/src/lib/notifications.ts")
      notificationModule = request.url();
  });
  await page.goto("http://127.0.0.1:1421");
  await expect(page.locator(".metrics-grid")).toBeVisible();
  expect(notificationModule).toBeTruthy();
  await page.evaluate(async (moduleUrl) => {
    const { message } = await import(/* @vite-ignore */ moduleUrl);
    (window as any).publishNotice = (tone: string) =>
      message("Fixture " + tone, tone, {
        duration: 10000,
        actionLabel: "Retry fixture",
        action: () => ((window as any).fixtureRetried = true),
      });
  }, notificationModule);
  const content = page.locator(".metrics-grid");
  const before = await content.boundingBox();
  for (const tone of ["info", "success", "warning", "error"]) {
    await page.evaluate((tone) => (window as any).publishNotice(tone), tone);
  }
  await expect(page.locator(".message-notification")).toHaveCount(4);
  expect((await content.boundingBox())!.y).toBe(before!.y);
  await expect(page.getByRole("alert")).toContainText("Fixture error");
  const message = page.locator(".message-notification.error");
  await message.hover();
  await page.clock.fastForward(15000);
  await expect(message).toBeVisible();
  await message.getByRole("button", { name: "Retry fixture" }).click();
  expect(await page.evaluate(() => (window as any).fixtureRetried)).toBe(true);
  await expect(message).toHaveCount(0);
  await page.evaluate(() => (window as any).publishNotice("success"));
  await page
    .locator(".message-notification.success")
    .getByRole("button", { name: "Dismiss notification" })
    .click();
  await expect(page.locator(".message-notification.success")).toHaveCount(0);
  expect((await content.boundingBox())!.y).toBe(before!.y);
  await expect(page.locator(".warning-banner, .toast")).toHaveCount(0);
});
