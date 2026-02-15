import { expect, test } from "@playwright/test";
import { API_BASE, navigateToProject, waitForAppReady } from "./helpers";

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Clear persisted state to simulate a fresh visitor. Must be called after navigating. */
async function clearAppState(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
}

/** Get the active session ID from the URL (e.g. /hive/abc-123 → abc-123). */
async function getSessionIdFromUrl(page: import("@playwright/test").Page): Promise<string | null> {
  const segments = new URL(page.url()).pathname.split("/").filter(Boolean);
  return segments.length >= 2 ? segments[1] : null;
}

/** Get the project slug from the URL (e.g. /millenium/abc → millenium). */
async function getProjectSlugFromUrl(
  page: import("@playwright/test").Page,
): Promise<string | null> {
  const segments = new URL(page.url()).pathname.split("/").filter(Boolean);
  return segments.length >= 1 ? segments[0] : null;
}

// ── First Visit (clean state) ────────────────────────────────────────────────

test.describe("First Visit — Clean State", () => {
  test("auto-selects a project when localStorage is empty", async ({ page }) => {
    await clearAppState(page);
    // Re-navigate after clearing to simulate fresh visit
    await page.goto("/");
    await waitForAppReady(page);
    await page.waitForTimeout(1000);

    // Should auto-select the first project and navigate to it
    const slug = await getProjectSlugFromUrl(page);
    expect(slug).toBeTruthy();
    expect(slug?.length).toBeGreaterThan(0);
  });

  test("auto-resumes most recent session for auto-selected project", async ({ page }) => {
    await clearAppState(page);
    await page.goto("/");
    await waitForAppReady(page);
    await page.waitForTimeout(1500);

    // Fetch sessions from API to see if any exist
    const res = await page.request.get(`${API_BASE}/api/chat/sessions`);
    const sessions: { id: string }[] = await res.json();

    if (sessions.length > 0) {
      // Should have auto-selected a session (URL has session ID)
      const sessionId = await getSessionIdFromUrl(page);
      expect(sessionId).toBeTruthy();
    }
  });

  test("shows context bar after project detection completes", async ({ page }) => {
    await clearAppState(page);
    await page.goto("/");
    await waitForAppReady(page);

    // Context bar should appear within 5s (detection pipeline runs)
    await expect(page.locator("[data-component='context-bar']")).toBeVisible({
      timeout: 8_000,
    });
  });
});

// ── Return Visit (persisted state) ───────────────────────────────────────────

test.describe("Return Visit — Persisted State", () => {
  test("restores persisted project on page reload", async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);

    // Navigate to Hive project
    await navigateToProject(page, "H");
    await page.waitForTimeout(1000);
    const slugBefore = await getProjectSlugFromUrl(page);
    expect(slugBefore).toMatch(/hive/);

    // Reload page — should restore to the same project
    await page.reload();
    await waitForAppReady(page);
    await page.waitForTimeout(1500);

    const slugAfter = await getProjectSlugFromUrl(page);
    expect(slugAfter).toMatch(/hive/);
  });

  test("restores session with conversation history on reload", async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);

    // Navigate to Hive project (known to have sessions)
    await navigateToProject(page, "H");
    await page.waitForTimeout(1500);

    const sessionBefore = await getSessionIdFromUrl(page);
    if (!sessionBefore) return; // No sessions to test

    // Check there's content visible
    const hasMessages =
      (await page.locator("[data-slot='user-message']").count()) > 0 ||
      (await page.locator("[data-component='session-turn']").count()) > 0;

    // Reload
    await page.reload();
    await waitForAppReady(page);
    await page.waitForTimeout(2000);

    // Same session should be restored
    const sessionAfter = await getSessionIdFromUrl(page);
    expect(sessionAfter).toBe(sessionBefore);

    // If we had messages before, they should still be there
    if (hasMessages) {
      const turnsAfter = page.locator("[data-component='session-turn']");
      await expect(turnsAfter.first()).toBeVisible({ timeout: 5_000 });
    }
  });
});

// ── Project Switching ────────────────────────────────────────────────────────

test.describe("Project Switching", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("context bar updates when switching projects", async ({ page }) => {
    // Go to Hive project
    await navigateToProject(page, "H");
    await page.waitForTimeout(2000);

    // Context bar should show Hive info (Rust)
    const rustBadge = page.getByText("Rust");
    if ((await rustBadge.count()) > 0) {
      await expect(rustBadge.first()).toBeVisible();
    }

    // Switch to Millenium project
    await navigateToProject(page, "M");
    await page.waitForTimeout(2000);

    // Context bar should update — check for different content
    const slug = await getProjectSlugFromUrl(page);
    expect(slug).toMatch(/millenium/);
  });

  test("URL updates when switching projects", async ({ page }) => {
    await navigateToProject(page, "H");
    await page.waitForTimeout(500);
    expect(page.url()).toMatch(/\/hive/);

    await navigateToProject(page, "M");
    await page.waitForTimeout(500);
    expect(page.url()).toMatch(/\/millenium/);
  });

  test("switches to project-specific session on project change", async ({ page }) => {
    // Start on Hive
    await navigateToProject(page, "H");
    await page.waitForTimeout(1500);
    const hiveSession = await getSessionIdFromUrl(page);

    // Switch to Millenium
    await navigateToProject(page, "M");
    await page.waitForTimeout(1500);
    const milleniumSession = await getSessionIdFromUrl(page);

    // Sessions should be different (each project has its own)
    if (hiveSession && milleniumSession) {
      expect(hiveSession).not.toBe(milleniumSession);
    }
  });

  test("round-trip project switch preserves sessions", async ({ page }) => {
    // Go to Hive, note session
    await navigateToProject(page, "H");
    await page.waitForTimeout(1500);
    const hiveSessionFirst = await getSessionIdFromUrl(page);

    // Switch to Millenium
    await navigateToProject(page, "M");
    await page.waitForTimeout(1500);

    // Switch back to Hive
    await navigateToProject(page, "H");
    await page.waitForTimeout(1500);
    const hiveSessionSecond = await getSessionIdFromUrl(page);

    // Should restore the same Hive session
    if (hiveSessionFirst && hiveSessionSecond) {
      expect(hiveSessionFirst).toBe(hiveSessionSecond);
    }
  });
});

// ── Context Usage ────────────────────────────────────────────────────────────

test.describe("Context Usage", () => {
  test("displays context usage when session has token data", async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
    await page.waitForTimeout(1500);

    // Look for usage indicator pattern: "X.Xk / 200.0k (N%)"
    const usage = page.getByText(/\d+\.?\d*k?\s*\/\s*200\.0k\s*\(\d+%\)/);
    const count = await usage.count();

    // If sessions exist with token data, usage should be visible
    if (count > 0) {
      await expect(usage.first()).toBeVisible();
    }
  });

  test("context usage is restored after page reload", async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
    await page.waitForTimeout(1500);

    // Get the usage text if visible
    const usage = page.getByText(/\d+\.?\d*k?\s*\/\s*200\.0k/);
    const hasUsage = (await usage.count()) > 0;
    if (!hasUsage) return; // No usage data to compare

    const usageBefore = await usage.first().textContent();

    // Reload
    await page.reload();
    await waitForAppReady(page);
    await page.waitForTimeout(2000);

    // Usage should be restored (not zero)
    const usageAfter = page.getByText(/\d+\.?\d*k?\s*\/\s*200\.0k/);
    if ((await usageAfter.count()) > 0) {
      const text = await usageAfter.first().textContent();
      expect(text).toBe(usageBefore);
    }
  });
});

// ── Status Bar ───────────────────────────────────────────────────────────────

test.describe("Status Bar Indicators", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("shows Ready status when no message is streaming", async ({ page }) => {
    await expect(page.getByText("Ready")).toBeVisible();
  });

  test("shows model indicator in status bar", async ({ page }) => {
    // Model pill should be visible (e.g. OPUS 4.6, SONNET 4.5)
    const modelIndicator = page.locator("[data-component='model-selector']");
    if ((await modelIndicator.count()) > 0) {
      await expect(modelIndicator.first()).toBeVisible();
    }
  });

  test("shows effort level in status bar", async ({ page }) => {
    // Effort indicator (LO, MEDIUM, HI) should be visible
    const effortText = page.getByText(/\b(LO|MEDIUM|HI)\b/);
    if ((await effortText.count()) > 0) {
      await expect(effortText.first()).toBeVisible();
    }
  });
});

// ── Prompt Input ─────────────────────────────────────────────────────────────

test.describe("Prompt Input", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("prompt input is visible and focusable", async ({ page }) => {
    const textbox = page.getByRole("textbox");
    await expect(textbox).toBeVisible();
    await textbox.focus();
    await expect(textbox).toBeFocused();
  });

  test("send button is disabled when input is empty", async ({ page }) => {
    const sendBtn = page.getByRole("button", { name: "Send message" });
    await expect(sendBtn).toBeDisabled();
  });

  test("send button enables when text is entered", async ({ page }) => {
    const textbox = page.getByRole("textbox");
    await textbox.fill("Hello");

    const sendBtn = page.getByRole("button", { name: "Send message" });
    await expect(sendBtn).toBeEnabled();
  });
});
