import { test, expect, type Page } from "@playwright/test";

// End-to-end coverage of the Reminders feature, driven entirely through
// the browser UI against a live lazyboy-server. Reminders are the cleanest
// full-stack slice: pure Lazyboy SQLite (no goose provider needed), with a
// complete create -> list -> dismiss lifecycle wired from the panel through
// RpcClient -> HTTP route -> repo and back.

// Each run tags its reminder with a unique body so a re-run never collides
// with rows a previous run left behind, and assertions can scope to the row
// this test created.
const tag = () => `pw-reminder-${process.hrtime.bigint()}`;

// A due time the UI's datetime-local input accepts: "YYYY-MM-DDTHH:mm",
// local time, comfortably in the future so the row reads "pending" not
// "overdue".
function futureLocalDateTime(): string {
  const d = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

async function openReminders(page: Page) {
  await page.goto("/", { waitUntil: "domcontentloaded" });
  // The space auto-selects from listSpaces(); wait for the panel tabs to
  // mount before reaching for the Reminders tab.
  await expect(page.locator('button[title="Reminders"]')).toBeVisible();
  await page.locator('button[title="Reminders"]').click();
  await expect(page.getByRole("heading", { name: /reminders/i })).toBeVisible();
}

// One reminder card, located by its body text. The card is the nearest
// ancestor that also holds the status badge and the Dismiss button.
function card(page: Page, body: string) {
  return page
    .locator("section", { hasText: /reminders/i })
    .locator("div", { has: page.getByText(body, { exact: true }) })
    .filter({ hasText: body })
    .last();
}

test("create a reminder through the UI and see it listed as pending", async ({ page }) => {
  const body = tag();
  await openReminders(page);

  await page.getByPlaceholder("Remind me to…").fill(body);
  await page.locator('input[type="datetime-local"]').fill(futureLocalDateTime());
  await page.getByRole("button", { name: "Add reminder" }).click();

  // The panel re-fetches after the create resolves; the new row appears
  // with a pending (future-dated) badge, not "overdue" or "dismissed".
  const row = card(page, body);
  await expect(row).toBeVisible();
  await expect(row).not.toContainText(/overdue|dismissed/i);
  await expect(row.getByRole("button", { name: "Dismiss" })).toBeVisible();

  // The create form clears on success.
  await expect(page.getByPlaceholder("Remind me to…")).toHaveValue("");
});

test("dismiss a reminder through the UI and see it settle", async ({ page }) => {
  const body = tag();
  await openReminders(page);

  await page.getByPlaceholder("Remind me to…").fill(body);
  await page.locator('input[type="datetime-local"]').fill(futureLocalDateTime());
  await page.getByRole("button", { name: "Add reminder" }).click();

  const row = card(page, body);
  await expect(row).toBeVisible();

  await row.getByRole("button", { name: "Dismiss" }).click();

  // After dismiss the row stays in the list but flips to the settled badge
  // and loses its Dismiss action.
  await expect(row).toContainText(/dismissed/i);
  await expect(row.getByRole("button", { name: "Dismiss" })).toHaveCount(0);
});

test("the Add button stays disabled until both body and due time are set", async ({ page }) => {
  await openReminders(page);

  const add = page.getByRole("button", { name: "Add reminder" });
  await expect(add).toBeDisabled();

  await page.getByPlaceholder("Remind me to…").fill(tag());
  // Body alone is not enough; a due time is required.
  await expect(add).toBeDisabled();

  await page.locator('input[type="datetime-local"]').fill(futureLocalDateTime());
  await expect(add).toBeEnabled();
});
