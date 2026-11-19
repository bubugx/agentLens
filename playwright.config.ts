import { defineConfig } from "@playwright/test";
export default defineConfig({
  webServer: {
    command: "npm run dev",
    url: "http://127.0.0.1:1421",
    reuseExistingServer: true,
  },
  use: {
    channel: "chrome",
    headless: true,
    viewport: { width: 1440, height: 960 },
  },
  timeout: 20000,
  testDir: "./tests",
  testMatch: "**/*.spec.ts",
});
