import { test as base } from "@playwright/test";

type AppFixtures = {
	navigateTo: (path: string) => Promise<void>;
};

export const test = base.extend<AppFixtures>({
	navigateTo: async ({ page }, use) => {
		await use(async (path: string) => {
			console.log(`Navigating to: ${path}`);
			await page.goto(path, { waitUntil: "domcontentloaded" });
			// Wait a short time for initial render instead of networkidle
			// (WebSocket stays connected so networkidle never happens)
			await page.waitForTimeout(500);
		});
	},
});

export { expect } from "@playwright/test";
