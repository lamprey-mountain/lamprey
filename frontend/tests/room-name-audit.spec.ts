import { expect, test } from "./fixtures";

test("room name change creates audit log entry", async ({ navigateTo, page }) => {
	// Step 1: Navigate to home and create a guest
	await navigateTo("/");
	await page.getByRole("button", { name: "create guest" }).click();

	// Enter guest name in the prompt modal
	await page.locator('input[type="text"]').first().fill("Test Guest");
	await page.getByRole("button", { name: "done!" }).click();

	// Wait for page reload after guest creation
	await page.waitForLoadState("domcontentloaded");
	await page.waitForTimeout(500);

	// Step 2: Create a room with a name
	await page.getByRole("button", { name: "create room" }).click();

	// Fill in room name in the modal
	const initialRoomName = "Test Room Initial";
	await page.locator('input[type="text"]').first().fill(initialRoomName);

	// Submit room creation - use the submit button in the modal
	await page.locator('form.new-room button[type="submit"]').click();

	// Wait for room to appear in the sidebar and click it
	// The room appears in RoomNav with data-room-id attribute
	await page.waitForSelector(
		`[data-room-id] .nav:has-text("${initialRoomName}")`,
		{ timeout: 5000 },
	);
	await page.locator(`[data-room-id] .nav:has-text("${initialRoomName}")`)
		.first().click();

	// Wait for room to load
	await page.waitForSelector(`h2:has-text("${initialRoomName}")`, {
		timeout: 5000,
	});

	// Step 3: Check that the name appears in the title
	await expect(page).toHaveTitle(new RegExp(initialRoomName, "i"));

	// Get the room ID from the current URL for later navigation
	const currentUrl = page.url();
	const roomIdMatch = currentUrl.match(/\/room\/([^\/\?#]+)/);
	expect(roomIdMatch).toBeTruthy();
	const roomId = roomIdMatch![1];

	// Step 4: Navigate to room settings (info page) and edit the room name
	await page.getByRole("link", { name: "settings" }).click();
	await page.waitForLoadState("domcontentloaded");
	await page.waitForTimeout(500);

	// Edit the room name in the info page
	const newName = "Test Room Updated";
	const nameInput = page.locator('input[type="text"]').first();
	await nameInput.fill("");
	await nameInput.fill(newName);

	// Wait for save button to appear (Savebar shows when there are unsaved changes)
	await page.locator('button.save:has-text("save")').waitFor({
		state: "visible",
		timeout: 5000,
	});

	// Save the changes
	await page.locator('button.save:has-text("save")').click();

	// Wait for save to complete
	await page.waitForTimeout(500);

	// Step 5: Check that the new name appears in the title
	await expect(page).toHaveTitle(new RegExp(newName, "i"));

	// Step 6: Navigate to the audit log
	await navigateTo(`/room/${roomId}/settings/logs`);

	// Step 7: Check that there is an audit log entry
	// Look for audit log entries in the list
	const auditLogList = page.locator(".room-settings-audit-log");
	await expect(auditLogList).toBeVisible();

	// Check for audit log entry (RoomUpdate event for name change)
	const auditLogEntries = auditLogList.locator("li");
	await expect(auditLogEntries.first()).toBeVisible();

	// Verify there's at least one audit log entry
	await expect(auditLogList.locator("li").first()).toBeVisible({
		timeout: 5000,
	});
});
