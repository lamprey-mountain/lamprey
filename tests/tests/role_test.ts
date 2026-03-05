import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Role Management and Permissions", async (t) => {
	const alice = await createTester("alice-role");
	const bob = await createTester("bob-role");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Role Test Room", public: false },
		status: 201,
	});
	const roomId = room.id;

	const channel = await alice({
		url: `/room/${roomId}/channel`,
		method: "POST",
		body: { name: "admin-only", type: "Text" },
		status: 201,
	});
	const channelId = channel.id;

	// Deny @everyone ViewChannel
	await alice({
		url: `/channel/${channelId}/permission/${roomId}`,
		method: "PUT",
		body: { type: "Role", allow: [], deny: ["ViewChannel"] },
		status: 204,
	});

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

	await t.step("Bob cannot see the admin channel", async () => {
		await bob({ url: `/channel/${channelId}`, status: 404 });
	});

	let roleId: string;

	await t.step("Alice creates a Moderator role", async () => {
		const role = await alice({
			url: `/room/${roomId}/role`,
			method: "POST",
			body: {
				name: "Moderator",
			},
			status: 201,
		});
		assertEquals(role.name, "Moderator");
		roleId = role.id;
	});

	await t.step("Alice gives Bob the Moderator role", async () => {
		await alice({
			url: `/room/${roomId}/role/${roleId}/member/${bob.user.id}`,
			method: "PUT",
			status: 200,
		});
	});

	await t.step("Bob can now see the admin channel via role", async () => {
		// FIXME: move this to separate step
		await alice({
			url: `/channel/${channelId}/permission/${roleId}`,
			method: "PUT",
			body: { type: "Role", allow: ["ViewChannel"], deny: [] },
			status: 204,
		});

		const channelInfo = await bob({
			url: `/channel/${channelId}`,
			status: 200,
		});
		assertEquals(channelInfo.id, channelId);
	});

	await t.step("Alice deletes the role", async () => {
		await alice({
			url: `/room/${roomId}/role/${roleId}?force=true`,
			method: "DELETE",
			status: 204,
		});

		// Bob should lose access again
		await bob({ url: `/channel/${channelId}`, status: 404 });
	});
});
