import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
	testDir: "./tests",
	timeout: 30 * 1000,
	expect: {
		timeout: 5000,
	},
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: process.env.CI ? 1 : undefined,

	use: {
		// support nixos
		acceptDownloads: false,
		launchOptions: {
			executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH,
		},
		baseURL: "http://localhost:5173",
		trace: "on-first-retry",
		screenshot: "only-on-failure",
		video: "retain-on-failure",
	},

	projects: [
		{ name: "chromium", use: { ...devices["Desktop Chrome"] } },
	],

	webServer: {
		command: "pnpm dev",
		port: 5173,
		reuseExistingServer: !process.env.CI,
		timeout: 120 * 1000,
	},
});
