import { createTester } from "../common.ts";

Deno.test("locked threads", async (t) => {
	const { tester: alice } = await createTester("alice-locked");
	const { tester: bob } = await createTester("bob-locked");
	const { tester: charlie, user: charlieUser } = await createTester(
		"charlie-locked",
	);

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "locked-test-room" },
		status: 201,
	});

	const invite = await alice({
		url: `/room/${room.id}/invite`,
		method: "POST",
		body: {},
		status: 201,
	});

	await bob({
		url: `/invite/${invite.code}`,
		method: "POST",
		status: 204,
	});
	await charlie({
		url: `/invite/${invite.code}`,
		method: "POST",
		status: 204,
	});

	const channel = await alice({
		url: `/room/${room.id}/channel`,
		method: "POST",
		body: { name: "locked-channel", type: "Text" },
		status: 201,
	});

	await t.step("bob can send message in unlocked channel", async () => {
		await bob({
			url: `/channel/${channel.id}/message`,
			method: "POST",
			body: { content: "hello" },
			status: 201,
		});
	});

	await t.step("alice locks the channel", async () => {
		await alice({
			url: `/channel/${channel.id}`,
			method: "PATCH",
			body: { locked: true },
			status: 200,
		});
	});

	await t.step("bob cannot send message in locked channel", async () => {
		await bob({
			url: `/channel/${channel.id}/message`,
			method: "POST",
			body: { content: "i should not be able to send this" },
			status: 403,
		});
	});

	await t.step("alice (owner) can send message in locked channel", async () => {
		await alice({
			url: `/channel/${channel.id}/message`,
			method: "POST",
			body: { content: "owner message" },
			status: 201,
		});
	});

	// Test user with ThreadLock permission
	const role = await alice({
		url: `/room/${room.id}/role`,
		method: "POST",
		body: { name: "moderator", permissions: ["ThreadLock"] },
		status: 201,
	});
	await alice({
		url: `/room/${room.id}/role/${role.id}/member/${charlieUser.id}`,
		method: "PUT",
		status: 200,
	});

	await t.step(
		"charlie (with ThreadLock) can send message in locked channel",
		async () => {
			await charlie({
				url: `/channel/${channel.id}/message`,
				method: "POST",
				body: { content: "moderator message" },
				status: 201,
			});
		},
	);

	await t.step("alice unlocks the channel", async () => {
		await alice({
			url: `/channel/${channel.id}`,
			method: "PATCH",
			body: { locked: false },
			status: 200,
		});
	});

	await t.step("bob can send message in unlocked channel again", async () => {
		await bob({
			url: `/channel/${channel.id}/message`,
			method: "POST",
			body: { content: "hello again" },
			status: 201,
		});
	});
});
