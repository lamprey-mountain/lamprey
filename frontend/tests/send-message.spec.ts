import { expect, test } from "./fixtures";

test("creating a text channel and sending messages with local echo", async ({ navigateTo, page }) => {
	// Step 1: Navigate to home and create a guest
	await navigateTo("/");
	await page.getByRole("button", { name: "create guest" }).click();

	// Enter guest name in the prompt modal
	await page.locator('input[type="text"]').first().fill("Test User");
	await page.getByRole("button", { name: "done!" }).click();

	// Wait for page reload after guest creation
	await page.waitForLoadState("domcontentloaded");
	await page.waitForTimeout(500);

	// Step 2: Create a room
	await page.getByRole("button", { name: "create room" }).click();

	// Fill in room name
	const roomName = "Message Test Room";
	await page.locator('input[type="text"]').first().fill(roomName);

	// Submit room creation
	await page.locator('form.new-room button[type="submit"]').click();

	// Wait for room to appear in the sidebar and click it
	await page.waitForSelector(`[data-room-id] .nav:has-text("${roomName}")`, {
		timeout: 5000,
	});
	await page.locator(`[data-room-id] .nav:has-text("${roomName}")`)
		.first()
		.click();

	// Wait for room to load
	await page.waitForSelector(`h2:has-text("${roomName}")`, {
		timeout: 5000,
	});

	// Step 3: Create a text channel by opening the room context menu
	// Right-click on the room header to open the context menu
	const roomHeader = page.locator("#channel-nav header.menu-room");
	await roomHeader.click({ button: "right" });

	// Wait for context menu to appear and click "create channel"
	await page.waitForSelector("text=create channel", { timeout: 5000 });
	await page.getByText("create channel").first().click();

	// Wait for modal to appear
	await page.waitForSelector("form.new-channel", { timeout: 5000 });

	// Fill in channel name
	const channelName = "general-test";
	await page.locator('form.new-channel input[type="text"]').first().fill(
		channelName,
	);

	// Text channel type is selected by default, so we can skip selecting it

	// Submit channel creation
	await page.locator('form.new-channel button[type="submit"]').click();

	// Wait for channel to appear in the channel nav and click it
	await page.waitForSelector(
		`[data-channel-id] .nav-channel:has-text("${channelName}")`,
		{ timeout: 5000 },
	);
	await page.locator(
		`[data-channel-id] .nav-channel:has-text("${channelName}")`,
	)
		.first()
		.click();

	// Wait for channel to load - look for the message input
	await page.waitForSelector(".message-input", { timeout: 5000 });

	// Step 4: Send a message
	const testMessage = "Hello, this is a test message!";
	const messageInput = page.locator(".message-input .text .ProseMirror");
	await messageInput.click();
	await messageInput.fill(testMessage);

	// Press Enter to send
	await page.keyboard.press("Enter");

	// Step 5: Verify the message appears with is_local class first
	// The message should appear quickly with the "local" class
	await page.waitForTimeout(100);

	// Find the message element - it should have the local class initially
	const messageLocator = page.locator(
		`article.message[data-message-id] .body.local:has-text("${testMessage}")`,
	);

	// Wait for the local message to appear
	await expect(messageLocator).toBeVisible({ timeout: 5000 });

	// Step 6: Wait for the server echo and verify is_local is removed
	// The local class should be removed once the server confirms the message
	// We wait for the message without the "local" class
	const confirmedMessageLocator = page.locator(
		`article.message[data-message-id] .body:not(.local):has-text("${testMessage}")`,
	);

	// Wait for the message to be confirmed (local class removed)
	await expect(confirmedMessageLocator).toBeVisible({ timeout: 10000 });

	// Verify the local version is gone
	await expect(messageLocator).not.toBeVisible({ timeout: 5000 });

	// Step 7: Send another message to verify consistency
	const secondMessage = "Second test message!";
	await messageInput.click();
	await messageInput.fill(secondMessage);
	await page.keyboard.press("Enter");

	// Wait for local message
	await page.waitForTimeout(100);
	const secondMessageLocal = page.locator(
		`article.message[data-message-id] .body.local:has-text("${secondMessage}")`,
	);

	// May or may not see local version depending on server speed
	// Wait for confirmed message
	const secondMessageConfirmed = page.locator(
		`article.message[data-message-id] .body:not(.local):has-text("${secondMessage}")`,
	);
	await expect(secondMessageConfirmed).toBeVisible({ timeout: 10000 });

	// Verify both messages are present without local class
	const allMessages = page.locator(
		"article.message[data-message-id] .body:not(.local)",
	);
	await expect(allMessages).toHaveCount(2, { timeout: 5000 });
});
