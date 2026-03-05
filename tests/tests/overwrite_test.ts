import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Permission Overwrites", async (t) => {
	const alice = await createTester("alice-ow");
	const bob = await createTester("bob-ow");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Overwrite Test Room", public: false },
		status: 201,
	});
	const roomId = room.id;

	const channel = await alice({
		url: `/room/${roomId}/channel`,
		method: "POST",
		body: { name: "secret-channel", type: "Text" },
		status: 201,
	});
	const channelId = channel.id;

	await t.step("Alice invites Bob", async () => {
		const invite = await alice({
			url: `/room/${roomId}/invite`,
			method: "POST",
			body: {},
			status: 201,
		});
		await bob({
			url: `/invite/${invite.code}`,
			method: "POST",
			status: 204,
		});
	});

	await t.step("Bob can see the channel initially", async () => {
		await bob({ url: `/channel/${channelId}`, status: 200 });
	});

	await t.step(
		"Alice denies ViewChannel for @everyone in this channel",
		async () => {
			// @everyone role id is the same as room id
			await alice({
				url: `/channel/${channelId}/permission/${roomId}`,
				method: "PUT",
				body: {
					type: "Role",
					allow: [],
					deny: ["ViewChannel"],
				},
				status: 204,
			});
		},
	);

	await t.step("Bob cannot see the channel anymore", async () => {
		await bob({ url: `/channel/${channelId}`, status: 404 });
	});

	await t.step("Alice allows ViewChannel for Bob specifically", async () => {
		await alice({
			url: `/channel/${channelId}/permission/${bob.user.id}`,
			method: "PUT",
			body: {
				type: "User",
				allow: ["ViewChannel"],
				deny: [],
			},
			status: 204,
		});
	});

	await t.step("Bob can see the channel again", async () => {
		await bob({ url: `/channel/${channelId}`, status: 200 });
	});
});
