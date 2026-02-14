import { expect, test } from "@playwright/test";
import {
  closeSettings,
  getCurrentTheme,
  openSettings,
  toggleTheme,
  waitForAppReady,
} from "./helpers";

test.describe("Settings Dialog", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("opens and closes settings dialog", async ({ page }) => {
    await openSettings(page);
    await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();

    await closeSettings(page);
    await expect(page.getByRole("dialog", { name: "Settings" })).not.toBeVisible();
  });

  test("General tab shows mode and theme options", async ({ page }) => {
    await openSettings(page);

    await expect(page.getByRole("tab", { name: "General" })).toBeVisible();
    // Light/Dark mode buttons
    await expect(page.getByRole("button", { name: "Light" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Dark", exact: true })).toBeVisible();
    // Color theme section
    await expect(page.getByText("Color theme")).toBeVisible();
  });

  test("shows all 9 theme options", async ({ page }) => {
    await openSettings(page);

    const themes = [
      "Hive",
      "Catppuccin",
      "Dracula",
      "Gruvbox",
      "One Dark",
      "Tokyo Night",
      "Monokai",
      "Flexoki",
      "Tron",
    ];
    for (const theme of themes) {
      await expect(page.getByRole("button", { name: `Select ${theme} theme` })).toBeVisible();
    }
  });

  test("font size controls are visible", async ({ page }) => {
    await openSettings(page);
    await expect(page.getByText(/Font size/)).toBeVisible();
    await expect(page.getByText("14px")).toBeVisible();
    await expect(page.getByRole("button", { name: "Decrease font size" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Increase font size" })).toBeVisible();
  });

  test("custom themes import/export buttons visible", async ({ page }) => {
    await openSettings(page);
    await expect(page.getByRole("button", { name: "Import" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Export current" })).toBeVisible();
  });

  test("Model tab shows available models", async ({ page }) => {
    await openSettings(page);
    await page.getByRole("tab", { name: "Model" }).click();

    const models = ["Claude Opus 4.6", "Claude Sonnet 4.5", "Claude Haiku 4.5"];
    for (const model of models) {
      await expect(page.getByRole("button", { name: model })).toBeVisible();
    }
  });

  test("Keybinds tab shows keyboard shortcuts", async ({ page }) => {
    await openSettings(page);
    await page.getByRole("tab", { name: "Keybinds" }).click();

    await expect(page.getByText("Session Management")).toBeVisible();
    await expect(page.getByText("Navigation")).toBeVisible();
    await expect(page.getByText("Editing")).toBeVisible();
    await expect(page.getByText("New session")).toBeVisible();
    await expect(page.getByText("Send message")).toBeVisible();
  });

  test("About tab shows version and tech stack", async ({ page }) => {
    await openSettings(page);
    await page.getByRole("tab", { name: "About" }).click();

    await expect(page.getByText("Version")).toBeVisible();
    await expect(page.getByText("0.1.0")).toBeVisible();
    await expect(page.getByText("Built with")).toBeVisible();
    await expect(page.getByText("React 19 + Tailwind v4 + Radix")).toBeVisible();
    await expect(page.getByText("Engine")).toBeVisible();
  });

  test("tabs can be switched sequentially", async ({ page }) => {
    await openSettings(page);

    const tabs = ["General", "Model", "Keybinds", "About"];
    for (const tab of tabs) {
      await page.getByRole("tab", { name: tab }).click();
      await expect(page.getByRole("tab", { name: tab, selected: true })).toBeVisible();
    }
  });
});

test.describe("Theme Toggle", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("toggles between dark and light mode", async ({ page }) => {
    const initial = await getCurrentTheme(page);

    await toggleTheme(page);
    const toggled = await getCurrentTheme(page);
    expect(toggled).not.toBe(initial);

    await toggleTheme(page);
    const restored = await getCurrentTheme(page);
    expect(restored).toBe(initial);
  });

  test("light mode applies correct background", async ({ page }) => {
    // Ensure dark mode first
    const theme = await getCurrentTheme(page);
    if (theme !== "dark") {
      await toggleTheme(page);
    }

    await toggleTheme(page);
    const lightTheme = await getCurrentTheme(page);
    expect(lightTheme).toBe("light");

    // Toggle back to dark
    await toggleTheme(page);
  });
});
