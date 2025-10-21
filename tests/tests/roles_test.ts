import { admin, createTester, SyncClient } from "../common.ts";
import { assertEquals } from "@std/assert";

Deno.test("role permissions", async (t) => {
	const { tester: regular, token: regularToken } = await createTester(
		"role-tester",
	);
	const regularUser = await regular({ url: "/user/@self", status: 200 });

	const room = await admin({
		url: "/room",
		method: "POST",
		body: { name: "role-test-room" },
		status: 201,
	});

	const invite = await admin({
		url: `/room/${room.id}/invite`,
		method: "POST",
		body: {},
		status: 201,
	});

	await regular({
		url: `/invite/${invite.code}`,
		method: "POST",
		status: 204,
	});

	const syncClient = new SyncClient(regularToken);
	await syncClient.connect();

	try {
		await t.step("user cannot create channel without permission", async () => {
			await regular({
				url: `/room/${room.id}/channel`,
				method: "POST",
				body: { name: "should-fail", type: "Text" },
				status: 403,
			});
		});

		let roleId: string;

		await t.step("admin creates a role and assigns it", async () => {
			const role = await admin({
				url: `/room/${room.id}/role`,
				method: "POST",
				body: {
					name: "channel-manager",
					permissions: ["ChannelManage"],
				},
				status: 201,
			});
			assertEquals(role.name, "channel-manager");
			roleId = role.id;

			await admin({
				url: `/room/${room.id}/role/${role.id}/member/${regularUser.id}`,
				method: "PUT",
				status: 200,
			});

			await syncClient.waitFor((msg) =>
				msg.type === "RoomMemberUpsert" &&
				msg.member.user_id === regularUser.id &&
				msg.member.roles.includes(role.id)
			);
		});

		await t.step("user can now create a channel", async () => {
			const channel = await regular({
				url: `/room/${room.id}/channel`,
				method: "POST",
				body: { name: "should-succeed", type: "Text" },
				status: 201,
			});
			assertEquals(channel.name, "should-succeed");
		});

		await t.step("admin removes role", async () => {
			await admin({
				url: `/room/${room.id}/role/${roleId}/member/${regularUser.id}`,
				method: "DELETE",
				status: 200,
			});

			await syncClient.waitFor((msg) =>
				msg.type === "RoomMemberUpsert" &&
				msg.member.user_id === regularUser.id &&
				!msg.member.roles.includes(roleId)
			);
		});

		await t.step("user can no longer create channels", async () => {
			await regular({
				url: `/room/${room.id}/channel`,
				method: "POST",
				body: { name: "should-fail-again", type: "Text" },
				status: 403,
			});
		});
	} finally {
		await syncClient.disconnect();
	}
});
