import { createTester } from "../common.ts";

Deno.test("permission overwrites", async (t) => {
	const { tester: alice } = await createTester("alice-overwrites");
	const { tester: bob, user: bobUser } = await createTester("bob-overwrites");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "overwrites-test-room" },
		status: 201,
	});

	const channel = await alice({
		url: `/room/${room.id}/channel`,
		method: "POST",
		body: { name: "overwrites-channel", type: "Text" },
		status: 201,
	});

	await t.step("bob cannot see channel without being in room", async () => {
		await bob({
			url: `/channel/${channel.id}`,
			method: "GET",
			status: 404,
		});
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

	await t.step("bob can see channel after joining room", async () => {
		await bob({
			url: `/channel/${channel.id}`,
			method: "GET",
			status: 200,
		});
	});

	const everyoneRole = room.id; // default role id is room id

	await t.step("alice denies ViewChannel for @everyone", async () => {
		await alice({
			url: `/channel/${channel.id}/permission/${everyoneRole}`,
			method: "PUT",
			body: {
				type: "Role",
				allow: [],
				deny: ["ViewChannel"],
			},
			status: 204,
		});
	});

	await t.step(
		"bob cannot see channel after ViewChannel is denied for @everyone",
		async () => {
			await bob({
				url: `/channel/${channel.id}`,
				method: "GET",
				status: 404,
			});
		},
	);

	await t.step("alice allows ViewChannel for bob specifically", async () => {
		await alice({
			url: `/channel/${channel.id}/permission/${bobUser.id}`,
			method: "PUT",
			body: {
				type: "User",
				allow: ["ViewChannel"],
				deny: [],
			},
			status: 204,
		});
	});

	await t.step("bob can see channel again with specific allow", async () => {
		await bob({
			url: `/channel/${channel.id}`,
			method: "GET",
			status: 200,
		});
	});
});
