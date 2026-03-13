import { expect, test } from "./fixtures";

/**
 * Helper function to set up a test environment with a guest, room, and text channel
 */
async function setupTestEnvironment(page: any) {
	// Create a guest
	await page.getByRole("button", { name: "create guest" }).click();
	await page.locator('input[type="text"]').first().fill("Test User");
	await page.getByRole("button", { name: "done!" }).click();

	// Wait for page reload after guest creation
	await page.waitForLoadState("domcontentloaded");

	// Create a room
	await page.getByRole("button", { name: "create room" }).click();
	const roomName = `Test Room ${Date.now()}`;
	await page.locator('input[type="text"]').first().fill(roomName);
	await page.locator('form.new-room button[type="submit"]').click();

	// Wait for room to appear and click it
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

	// Create a text channel by right-clicking on the room header
	const roomHeader = page.locator("#channel-nav header.menu-room");
	await roomHeader.click({ button: "right" });

	// Wait for context menu and click "create channel"
	await page.waitForSelector("text=create channel", { timeout: 5000 });
	await page.getByText("create channel").first().click();

	// Wait for modal and fill in channel name
	await page.waitForSelector("form.new-channel", { timeout: 5000 });
	const channelName = "general-test";
	await page.locator('form.new-channel input[type="text"]').first().fill(
		channelName,
	);

	// Submit channel creation
	await page.locator('form.new-channel button[type="submit"]').click();

	// Wait for channel to appear and click it
	await page.waitForSelector(
		`[data-channel-id] .nav-channel:has-text("${channelName}")`,
		{ timeout: 5000 },
	);
	await page.locator(
		`[data-channel-id] .nav-channel:has-text("${channelName}")`,
	)
		.first()
		.click();

	// Wait for channel to load
	await page.waitForSelector(".message-input", { timeout: 5000 });

	return { roomName, channelName };
}

/**
 * Helper function to get the message input element
 */
function getMessageInput(page: any) {
	return page.locator(".message-input .text .ProseMirror");
}

test.describe("Message Sending", () => {
	test("basic message sending with is_local echo", async ({
		navigateTo,
		page,
	}) => {
		// Set up test environment
		await navigateTo("/");
		await setupTestEnvironment(page);

		// Send a message
		const testMessage = "Hello, this is a test message!";
		const messageInput = getMessageInput(page);
		await messageInput.click();
		await messageInput.fill(testMessage);
		await page.keyboard.press("Enter");

		// Verify the message appears with is_local class first (local echo)
		const localMessageLocator = page.locator(
			`article.message[data-message-id] .body.local:has-text("${testMessage}")`,
		);

		// The local message may or may not be visible depending on server speed
		// but we verify the confirmed message appears without local class
		const confirmedMessageLocator = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("${testMessage}")`,
		);

		// Wait for the message to be confirmed (local class removed)
		await expect(confirmedMessageLocator).toBeVisible({ timeout: 10000 });

		// Verify the local version is gone (should not exist anymore)
		await expect(localMessageLocator).not.toBeVisible({ timeout: 5000 });
	});

	test("multiple messages maintain is_local lifecycle", async ({
		navigateTo,
		page,
	}) => {
		// Set up test environment
		await navigateTo("/");
		await setupTestEnvironment(page);

		const messageInput = getMessageInput(page);

		// Send first message
		const firstMessage = "First test message";
		await messageInput.click();
		await messageInput.fill(firstMessage);
		await page.keyboard.press("Enter");

		// Wait for first message to be confirmed
		const firstConfirmed = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("${firstMessage}")`,
		);
		await expect(firstConfirmed).toBeVisible({ timeout: 10000 });

		// Send second message
		const secondMessage = "Second test message";
		await messageInput.click();
		await messageInput.fill(secondMessage);
		await page.keyboard.press("Enter");

		// Wait for second message to be confirmed
		const secondConfirmed = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("${secondMessage}")`,
		);
		await expect(secondConfirmed).toBeVisible({ timeout: 10000 });

		// Verify both messages are present without local class
		const allMessages = page.locator(
			"article.message[data-message-id] .body:not(.local)",
		);
		await expect(allMessages).toHaveCount(2, { timeout: 5000 });
	});
});

test.describe("Markdown Rendering", () => {
	test("bold markdown renders correctly", async ({ navigateTo, page }) => {
		await navigateTo("/");
		await setupTestEnvironment(page);

		const testMessage = "**bold text**";
		const messageInput = getMessageInput(page);
		await messageInput.click();
		await messageInput.fill(testMessage);
		await page.keyboard.press("Enter");

		// Wait for message to be confirmed
		const confirmedMessage = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("bold text")`,
		);
		await expect(confirmedMessage).toBeVisible({ timeout: 10000 });

		// Verify bold element is rendered
		const boldElement = page.locator(
			`article.message[data-message-id] .body:not(.local) strong:has-text("bold text")`,
		);
		await expect(boldElement).toBeVisible({ timeout: 5000 });
	});

	test("italic markdown renders correctly", async ({ navigateTo, page }) => {
		await navigateTo("/");
		await setupTestEnvironment(page);

		const testMessage = "*italic text*";
		const messageInput = getMessageInput(page);
		await messageInput.click();
		await messageInput.fill(testMessage);
		await page.keyboard.press("Enter");

		// Wait for message to be confirmed
		const confirmedMessage = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("italic text")`,
		);
		await expect(confirmedMessage).toBeVisible({ timeout: 10000 });

		// Verify italic element is rendered
		const italicElement = page.locator(
			`article.message[data-message-id] .body:not(.local) em:has-text("italic text")`,
		);
		await expect(italicElement).toBeVisible({ timeout: 5000 });
	});

	test("code block markdown renders correctly", async ({ navigateTo, page }) => {
		await navigateTo("/");
		await setupTestEnvironment(page);

		const testMessage = "`inline code`";
		const messageInput = getMessageInput(page);
		await messageInput.click();
		await messageInput.fill(testMessage);
		await page.keyboard.press("Enter");

		// Wait for message to be confirmed
		const confirmedMessage = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("inline code")`,
		);
		await expect(confirmedMessage).toBeVisible({ timeout: 10000 });

		// Verify code element is rendered
		const codeElement = page.locator(
			`article.message[data-message-id] .body:not(.local) code:has-text("inline code")`,
		);
		await expect(codeElement).toBeVisible({ timeout: 5000 });
	});

	test("link markdown renders correctly", async ({ navigateTo, page }) => {
		await navigateTo("/");
		await setupTestEnvironment(page);

		const testMessage = "[example link](https://example.com)";
		const messageInput = getMessageInput(page);
		await messageInput.click();
		await messageInput.fill(testMessage);
		await page.keyboard.press("Enter");

		// Wait for message to be confirmed
		const confirmedMessage = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("example link")`,
		);
		await expect(confirmedMessage).toBeVisible({ timeout: 10000 });

		// Verify link element is rendered with correct href
		const linkElement = page.locator(
			`article.message[data-message-id] .body:not(.local) a[href="https://example.com"]`,
		);
		await expect(linkElement).toBeVisible({ timeout: 5000 });
	});
});

test.describe("Message Editing", () => {
	test("edit message via right-click context menu", async ({
		navigateTo,
		page,
	}) => {
		await navigateTo("/");
		await setupTestEnvironment(page);

		// Send initial message
		const originalMessage = "Original message " + Date.now();
		const messageInput = getMessageInput(page);
		await messageInput.click();
		await messageInput.fill(originalMessage);
		await page.keyboard.press("Enter");

		// Wait for message to be confirmed
		const confirmedMessage = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("${originalMessage}")`,
		);
		await expect(confirmedMessage).toBeVisible({ timeout: 10000 });

		// Get the message element to right-click on
		const messageArticle = page.locator(
			`article.message[data-message-id]:has-text("${originalMessage}")`,
		).first();

		// Ensure any open menu is closed first
		await page.keyboard.press("Escape");
		await page.waitForTimeout(200);

		// Right-click on the message to open context menu
		await messageArticle.click({ button: "right" });

		// Wait for context menu and click edit
		const editMenuItem = page.locator('[role="menu"] >> text=edit').first();
		await expect(editMenuItem).toBeVisible({ timeout: 5000 });
		await editMenuItem.click();

		// Wait for editor to appear
		const messageEditor = page.locator(".message-editor");
		await expect(messageEditor).toBeVisible({ timeout: 10000 });

		// Fill in the edited message
		const editedMessage = "Edited message content";
		const editor = messageEditor.locator("[contenteditable]");
		await editor.click();
		await editor.fill(editedMessage);

		// Press Enter to save the edit
		await page.keyboard.press("Enter");

		// Wait for the edited message to be confirmed (without local class)
		const editedConfirmed = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("${editedMessage}")`,
		);
		await expect(editedConfirmed).toBeVisible({ timeout: 10000 });

		// Verify the original message text is no longer present
		const originalGone = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("${originalMessage}")`,
		);
		await expect(originalGone).not.toBeVisible({ timeout: 5000 });

		// Verify the "(edited)" indicator is present
		const editedIndicator = page.locator(
			`article.message[data-message-id] .edited:has-text("(edited)")`,
		);
		await expect(editedIndicator).toBeVisible({ timeout: 5000 });
	});

	test("edit message preserves is_local lifecycle during edit", async ({
		navigateTo,
		page,
	}) => {
		await navigateTo("/");
		await setupTestEnvironment(page);

		// Send initial message
		const originalMessage = "Message to edit " + Date.now();
		const messageInput = getMessageInput(page);
		await messageInput.click();
		await messageInput.fill(originalMessage);
		await page.keyboard.press("Enter");

		// Wait for message to be confirmed
		const confirmedMessage = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("${originalMessage}")`,
		);
		await expect(confirmedMessage).toBeVisible({ timeout: 10000 });

		// Get the message article element to right-click on
		const messageArticle = page.locator(
			`article.message[data-message-id]:has-text("${originalMessage}")`,
		).first();
		
		// Ensure any open menu is closed first
		await page.keyboard.press("Escape");
		await page.waitForTimeout(200);
		
		// Right-click on the message to open context menu
		await messageArticle.click({ button: "right" });

		// Wait for context menu and click edit
		const editMenuItem = page.locator('[role="menu"] >> text=edit').first();
		await expect(editMenuItem).toBeVisible({ timeout: 5000 });
		await editMenuItem.click();

		// Wait for editor to appear
		const messageEditor = page.locator(".message-editor");
		await expect(messageEditor).toBeVisible({ timeout: 10000 });

		// Edit the message
		const editedMessage = "Updated content";
		const editor = messageEditor.locator("[contenteditable]");
		await editor.click();
		await editor.fill(editedMessage);
		await page.keyboard.press("Enter");

		// During edit, the message may temporarily have is_local class
		// but should eventually be confirmed without it
		const localEditLocator = page.locator(
			`article.message[data-message-id] .body.local:has-text("${editedMessage}")`,
		);

		// May or may not see local version depending on server speed
		// Wait for the confirmed edit (without local class)
		const confirmedEdit = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("${editedMessage}")`,
		);
		await expect(confirmedEdit).toBeVisible({ timeout: 10000 });

		// Verify local version is gone
		await expect(localEditLocator).not.toBeVisible({ timeout: 5000 });
	});

	test("cancel edit by pressing Escape", async ({ navigateTo, page }) => {
		await navigateTo("/");
		await setupTestEnvironment(page);

		// Send initial message
		const originalMessage = "Original message " + Date.now();
		const messageInput = getMessageInput(page);
		await messageInput.click();
		await messageInput.fill(originalMessage);
		await page.keyboard.press("Enter");

		// Wait for message to be confirmed
		const confirmedMessage = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("${originalMessage}")`,
		);
		await expect(confirmedMessage).toBeVisible({ timeout: 10000 });

		// Get the message article element to right-click on
		const messageArticle = page.locator(
			`article.message[data-message-id]:has-text("${originalMessage}")`,
		).first();

		// Ensure any open menu is closed first
		await page.keyboard.press("Escape");
		await page.waitForTimeout(200);

		// Right-click to open context menu
		await messageArticle.click({ button: "right" });

		// Wait for context menu and click edit
		const editMenuItem = page.locator('[role="menu"] >> text=edit').first();
		await expect(editMenuItem).toBeVisible({ timeout: 5000 });
		await editMenuItem.click();

		// Wait for editor to appear
		const messageEditor = page.locator(".message-editor");
		await expect(messageEditor).toBeVisible({ timeout: 10000 });

		// Start editing but then cancel
		const editor = messageEditor.locator("[contenteditable]");
		await editor.click();
		await editor.fill("Changed my mind");

		// Press Escape to cancel
		await page.keyboard.press("Escape");

		// Verify original message is still there
		await expect(confirmedMessage).toBeVisible({ timeout: 5000 });

		// Verify the changed text is not present
		const changedGone = page.locator(
			`article.message[data-message-id] .body:not(.local):has-text("Changed my mind")`,
		);
		await expect(changedGone).not.toBeVisible({ timeout: 5000 });

		// Verify no (edited) indicator since we cancelled
		const editedIndicator = page.locator(
			`article.message[data-message-id] .edited:has-text("(edited)")`,
		);
		await expect(editedIndicator).not.toBeVisible({ timeout: 5000 });
	});
});
