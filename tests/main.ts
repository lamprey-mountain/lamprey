import { assertEquals } from "@std/assert";

const BASE_URL = `${Deno.env.get("BASE_URL")}/api/v1`;
const USER0 = Deno.env.get("TOKEN_U0");
const USER1 = Deno.env.get("TOKEN_U1");

type TesterConfig = { token: string; who: string };

type TesterRequest = {
	url: string;
	method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
	body?: BodyInit;
	status: number;
};

const makeTester =
	({ token, who }: TesterConfig) => async (cfg: TesterRequest) => {
		const { url, method, body, status } = cfg;
		console.log(`${who}: ${method ?? "GET"} ${url}`);
		const res = await fetch(`${BASE_URL}${url}`, {
			headers: {
				"authorization": `Bearer ${token}`,
				"content-type": "application/json",
			},
			method,
			body,
		});
		const data = res.status === 204 ? null : await res.json();
		assertEquals(res.status, status);
		return data;
	};

const room_id = "0194f5be-a340-7011-b7c9-191a29ff5658";
const role_id = "0194f5be-a34b-75e3-9738-d6bd6dd5764c";
const u1_id = "0194f5ba-89e0-7c62-8776-8873b37a6d0a";

const f0 = makeTester({ token: USER0, who: "user0" });
const f1 = makeTester({ token: USER1, who: "user1" });

async function testPermissions() {
	console.log("testing permissions");

	const thread = await f0({
		url: `/room/${room_id}/thread`,
		method: "POST",
		body: JSON.stringify({ name: "testing" }),
		status: 201,
	});

	const tid = thread.id;

	await f1({ url: `/thread/${tid}`, method: "DELETE", status: 403 });

	await f0({
		url: `/room/${room_id}/role/${role_id}/member/${u1_id}`,
		method: "PUT",
		status: 200,
	});

	await f1({ url: `/thread/${tid}`, method: "DELETE", status: 204 });

	const thread2 = await f0({
		url: `/room/${room_id}/thread`,
		method: "POST",
		body: JSON.stringify({ name: "testing" }),
		status: 201,
	});
	const tid2 = thread2.id;

	await f0({
		url: `/room/${room_id}/role/${role_id}/member/${u1_id}`,
		method: "DELETE",
		status: 200,
	});

	await f1({ url: `/thread/${tid2}`, method: "DELETE", status: 403 });

	await f0({ url: `/thread/${tid2}`, method: "DELETE", status: 204 });
}

testPermissions();
