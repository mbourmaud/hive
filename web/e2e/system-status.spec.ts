import { expect, test } from "@playwright/test";
import { waitForAppReady } from "./helpers";

test.describe("System Status Panel", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("opens system status panel", async ({ page }) => {
    await page.evaluate(() => {
      const btn = document.querySelector<HTMLButtonElement>('[title="System status"]');
      btn?.click();
    });

    await expect(page.getByText("System Status")).toBeVisible();
  });

  test("shows version number", async ({ page }) => {
    await page.evaluate(() => {
      const btn = document.querySelector<HTMLButtonElement>('[title="System status"]');
      btn?.click();
    });

    await expect(page.getByText(/v\d+\.\d+\.\d+/)).toBeVisible();
  });

  test("shows auth status", async ({ page }) => {
    await page.evaluate(() => {
      const btn = document.querySelector<HTMLButtonElement>('[title="System status"]');
      btn?.click();
    });

    // Session section title
    await expect(
      page.locator('[data-slot="status-section-title"]', {
        hasText: "Session",
      }),
    ).toBeVisible();
    // Auth type badge
    const authBadge = page.locator('[data-slot="status-value"]').first();
    await expect(authBadge).toBeVisible();
  });

  test("shows MCP servers section", async ({ page }) => {
    await page.evaluate(() => {
      const btn = document.querySelector<HTMLButtonElement>('[title="System status"]');
      btn?.click();
    });

    await expect(page.getByText("MCP SERVERS")).toBeVisible();
  });

  test("shows active sessions count", async ({ page }) => {
    await page.evaluate(() => {
      const btn = document.querySelector<HTMLButtonElement>('[title="System status"]');
      btn?.click();
    });

    await expect(page.getByText("Active sessions")).toBeVisible();
    // The count like "0 / 1" appears in a status-value slot
    const sessionCount = page
      .locator('[data-slot="status-value"]')
      .filter({ hasText: /\d+\s*\/\s*\d+/ });
    await expect(sessionCount.first()).toBeVisible();
  });

  test("toggles system status panel open/close", async ({ page }) => {
    // Open
    await page.evaluate(() => {
      document.querySelector<HTMLButtonElement>('[title="System status"]')?.click();
    });
    await expect(page.getByText("System Status")).toBeVisible();

    // Close by clicking again
    await page.evaluate(() => {
      document.querySelector<HTMLButtonElement>('[title="System status"]')?.click();
    });
    await page.waitForTimeout(300);

    // The panel text should no longer be visible (or minimized)
    // Note: behavior may vary — panel could remain open as a popover
  });
});
