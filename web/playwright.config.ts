import { defineConfig } from "@playwright/test";

export default defineConfig({
	testDir: "./e2e",
	timeout: 30_000,
	expect: { timeout: 5_000 },
	fullyParallel: false,
	retries: 1,
	workers: 1,
	reporter: "html",
	use: {
		baseURL: "http://localhost:5173",
		trace: "on-first-retry",
		screenshot: "only-on-failure",
	},
	webServer: {
		command: "npx vite --port 5173",
		port: 5173,
		reuseExistingServer: true,
		timeout: 15_000,
	},
});
