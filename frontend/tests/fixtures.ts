import { test as base } from "@playwright/test";

type AppFixtures = {
	navigateTo: (path: string) => Promise<void>;
};

export const test = base.extend<AppFixtures>({
	navigateTo: async ({ page }, use) => {
		await use(async (path: string) => {
			console.log(`Navigating to: ${path}`);
			await page.goto(path);
			await page.waitForLoadState("networkidle");
		});
	},
});

export { expect } from "@playwright/test";
