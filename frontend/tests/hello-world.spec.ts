import { expect, test } from "./fixtures";

test("homepage loads", async ({ navigateTo, page }) => {
	await navigateTo("/");
	await expect(page).toHaveTitle("Home");
});
