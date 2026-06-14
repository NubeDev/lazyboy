import { defineConfig, devices } from "@playwright/test";

// Browser-driven UI tests. They run against a LIVE stack — the
// lazyboy-server (HTTP+SSE core) and the Vite dev server — not a mock.
// Bring both up before running (see testing/README.md); the tests do not
// spawn them, so a failure points at the real app, not at fixture drift.
//
// The app polls on 15s/60s timers (health, "now"), so the network never
// goes idle; tests wait on DOM state, never on `networkidle`.
export default defineConfig({
  testDir: ".",
  testMatch: "*.spec.ts",
  // Reminders share one space's list; serialize so a parallel run cannot
  // dismiss a row another test is asserting on.
  fullyParallel: false,
  workers: 1,
  reporter: [["list"]],
  use: {
    baseURL: process.env.UI_URL ?? "http://localhost:5181",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
