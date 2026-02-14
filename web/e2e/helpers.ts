import { expect, type Page } from "@playwright/test";

/**
 * Base URL for the Hive backend API.
 * The Vite dev server proxies /api/* to this.
 */
export const API_BASE = "http://localhost:3333";

/**
 * Wait for the app to fully hydrate by checking for the navigation bar.
 */
export async function waitForAppReady(page: Page): Promise<void> {
  await page.waitForSelector('img[alt="Hive"]', { timeout: 10_000 });
}

/**
 * Navigate to a specific project by clicking its sidebar button.
 */
export async function navigateToProject(page: Page, initial: string): Promise<void> {
  await page.getByRole("button", { name: initial, exact: true }).click();
  await page.waitForTimeout(500);
}

/**
 * Open the Settings dialog.
 */
export async function openSettings(page: Page): Promise<void> {
  await page.evaluate(() => {
    const btn = document.querySelector<HTMLButtonElement>('[title="Settings"]');
    btn?.click();
  });
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
}

/**
 * Close the Settings dialog.
 */
export async function closeSettings(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Close settings" }).click({ force: true });
  await expect(page.getByRole("dialog", { name: "Settings" })).not.toBeVisible();
}

/**
 * Toggle the light/dark theme using JS (avoids overlay interception).
 */
export async function toggleTheme(page: Page): Promise<void> {
  await page.evaluate(() => {
    const btn = document.querySelector<HTMLButtonElement>('[title="Toggle light/dark theme"]');
    btn?.click();
  });
  await page.waitForTimeout(300);
}

/**
 * Get the current data-theme attribute on <html>.
 */
export async function getCurrentTheme(page: Page): Promise<string | null> {
  return page.evaluate(() => document.documentElement.dataset.theme ?? null);
}

/**
 * Ensure the API server is reachable and projects registry is valid.
 */
export async function ensureApiReady(page: Page): Promise<void> {
  const res = await page.request.get(`${API_BASE}/api/auth/status`);
  expect(res.ok()).toBe(true);
}
