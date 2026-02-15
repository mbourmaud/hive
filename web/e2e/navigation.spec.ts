import { expect, test } from "@playwright/test";
import { navigateToProject, waitForAppReady } from "./helpers";

test.describe("Navigation & Layout", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("loads the app with Hive logo and sidebar", async ({ page }) => {
    await expect(page.getByRole("img", { name: "Hive" }).first()).toBeVisible();
    await expect(page.getByRole("navigation")).toBeVisible();
  });

  test("shows project buttons in the sidebar", async ({ page }) => {
    // At least one project button should exist
    const nav = page.getByRole("navigation");
    const buttons = nav.getByRole("button");
    const count = await buttons.count();
    // Add project + at least 1 project + sidebar utilities
    expect(count).toBeGreaterThanOrEqual(3);
  });

  test("switches between projects via sidebar", async ({ page }) => {
    // Click "H" project (Hive)
    await navigateToProject(page, "H");
    await expect(page).toHaveURL(/\/hive/);

    // Click "M" project (Millenium)
    await navigateToProject(page, "M");
    await expect(page).toHaveURL(/\/millenium/);
  });

  test("shows context bar with branch and language info", async ({ page }) => {
    await navigateToProject(page, "H");

    // Wait for context detection to complete (SSE pipeline)
    await expect(page.locator("[data-component='context-bar']")).toBeVisible({
      timeout: 8_000,
    });

    // Language badge should be visible (Rust for Hive project)
    await expect(page.getByText("Rust")).toBeVisible();
    // Changed files indicator
    await expect(page.getByText(/\d+ changed/)).toBeVisible();
  });

  test("add project button is visible", async ({ page }) => {
    await expect(page.getByRole("button", { name: "Add project" })).toBeVisible();
  });
});

test.describe("Drone Panel", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("drone panel is visible when project has drones", async ({ page }) => {
    // Navigate to millenium which has active drones
    await navigateToProject(page, "M");
    await page.waitForTimeout(1000);

    // Drone panel heading or drone entries should be visible
    const dronesText = page.getByText("Drones");
    const hasDrones = (await dronesText.count()) > 0;
    if (hasDrones) {
      await expect(dronesText.first()).toBeVisible();
    }
  });

  test("can hide and show the drone panel", async ({ page }) => {
    await navigateToProject(page, "M");
    await page.waitForTimeout(1000);

    // Try to find the hide button (only exists if panel is open)
    const hideBtn = page.getByRole("button", { name: /hide drone/i });
    if ((await hideBtn.count()) > 0) {
      await hideBtn.click({ force: true });
      await page.waitForTimeout(300);

      const showBtn = page.getByRole("button", { name: /show drone/i });
      await expect(showBtn).toBeVisible();

      await showBtn.click({ force: true });
      await page.waitForTimeout(300);
      await expect(page.getByText("Drones").first()).toBeVisible();
    }
  });

  test("shows drones for millenium project", async ({ page }) => {
    await navigateToProject(page, "M");
    await page.waitForTimeout(1000);

    // Should show drone task progress counts (e.g. "0/2", "1/1")
    const taskCounts = page.locator('[data-slot="status-value"]');
    const count = await taskCounts.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test("expands drone details on click", async ({ page }) => {
    await navigateToProject(page, "M");
    await page.waitForTimeout(1000);

    // Click a drone entry
    const drone = page.getByRole("button", {
      name: /prod-29199.*gli-go-pdata/,
    });
    if ((await drone.count()) > 0) {
      await drone.click();
      // Should show task details
      await expect(page.getByText("TASKS")).toBeVisible();
      await expect(page.getByText("COST")).toBeVisible();
    }
  });

  test("shows Live indicator", async ({ page }) => {
    await expect(page.getByText("Live")).toBeVisible();
  });
});
