import { assertEquals } from "@std/assert";

export const BASE_URL = `${Deno.env.get("BASE_URL")}/api/v1`;
export const TOKEN = Deno.env.get("TOKEN");

if (!TOKEN) {
	throw new Error("TOKEN must be set in the environment.");
}

export type TesterConfig = { token: string; name: string };

export type TesterRequest = {
	url: string;
	method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
	body?: any;
	status: number;
};

export const makeTester =
	({ token, name: who }: TesterConfig) => async (cfg: TesterRequest) => {
		const { url, method, body, status } = cfg;
		console.log(`${who}: ${method ?? "GET"} ${url}`);
		const res = await fetch(`${BASE_URL}${url}`, {
			headers: {
				"authorization": `Bearer ${token}`,
				"content-type": "application/json",
			},
			method: method ?? "GET",
			body: body ? JSON.stringify(body) : undefined,
		});
		const data = res.status === 204 ? null : await res.json();
		assertEquals(res.status, status);
		return data;
	};

export const admin = makeTester({ token: TOKEN!, name: "admin" });

const createdTesters: Record<
	string,
	{ tester: ReturnType<typeof makeTester>; token: string; user: any }
> = {};

export async function createTester(
	name: string,
): Promise<
	{ tester: ReturnType<typeof makeTester>; token: string; user: any }
> {
	if (createdTesters[name]) {
		return createdTesters[name];
	}

	const session = await fetch(`${BASE_URL}/session`, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ name: `temp-session-for-${name}` }),
	}).then((r) => r.json());

	const guestUser = await makeTester({
		token: session.token,
		name: `guest-${name}`,
	})({
		url: "/guest",
		method: "POST",
		body: { name },
		status: 201,
	});

	await admin({
		url: "/admin/register-user",
		method: "POST",
		body: { user_id: guestUser.id },
		status: 204,
	});

	const tester = makeTester({ token: session.token, name });
	const user = await tester({ url: "/user/@self", status: 200 });

	const result = { tester, token: session.token, user };
	createdTesters[name] = result;
	return result;
}

const syncClients: Record<string, SyncClient> = {};

export class SyncClient {
	private ws: WebSocket;
	private receivedMessages: any[] = [];
	private waiters: {
		predicate: (msg: any) => boolean;
		resolve: (msg: any) => void;
		timer: number;
	}[] = [];
	private ready: Promise<void>;

	constructor(token: string) {
		const url = new URL(Deno.env.get("BASE_URL")!);
		const wsUrl = `${
			url.protocol === "https:" ? "wss:" : "ws:"
		}//${url.host}/api/v1/sync?version=1`;
		this.ws = new WebSocket(wsUrl);
		let resolve: () => void;

		this.ready = new Promise((r) => {
			resolve = r;
			this.ws.onopen = () => {
				this.ws.send(JSON.stringify({
					type: "Hello",
					token: token,
				}));
			};
		});

		this.ws.onmessage = (event) => {
			const msg = JSON.parse(event.data);
			if (msg.op === "Ready") {
				resolve();
				return;
			}
			if (msg.op === "Sync") {
				const waiterIndex = this.waiters.findIndex((w) =>
					w.predicate(msg.data)
				);
				if (waiterIndex !== -1) {
					const [waiter] = this.waiters.splice(waiterIndex, 1);
					clearTimeout(waiter.timer);
					waiter.resolve(msg.data);
				} else {
					this.receivedMessages.push(msg.data);
				}
			}
		};
	}

	async connect() {
		return this.ready;
	}

	waitFor(predicate: (msg: any) => boolean, timeout = 5000): Promise<any> {
		return new Promise((resolve, reject) => {
			const index = this.receivedMessages.findIndex(predicate);
			if (index !== -1) {
				const [msg] = this.receivedMessages.splice(index, 1);
				resolve(msg);
				return;
			}

			const timer = setTimeout(
				() => {
					const waiterIndex = this.waiters.findIndex((w) =>
						w.predicate === predicate
					);
					if (waiterIndex !== -1) {
						this.waiters.splice(waiterIndex, 1);
					}
					reject(
						new Error(
							`Timeout waiting for message with predicate: ${predicate.toString()}`,
						),
					);
				},
				timeout,
			);

			this.waiters.push({
				predicate,
				resolve: (msg) => {
					clearTimeout(timer);
					resolve(msg);
				},
				timer,
			});
		});
	}

	disconnect(): Promise<void> {
		return new Promise((resolve) => {
			if (this.ws.readyState === WebSocket.CLOSED) {
				resolve();
				return;
			}
			this.ws.onclose = () => resolve();
			this.ws.close();
		});
	}
}

export async function getSyncClient(token: string): Promise<SyncClient> {
	if (syncClients[token]) {
		return syncClients[token];
	}
	const client = new SyncClient(token);
	await client.connect();
	syncClients[token] = client;
	return client;
}
