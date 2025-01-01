import { UUID } from "uuidv7";

export type Data = {
	rooms: Record<string, any>;
	threads: Record<string, Thread>;
	messages: Record<string, any>;
	// users: Record<string, any>,
	user: any;
};

export const blankData: Data = {
	rooms: {},
	threads: {},
	messages: {},
	user: null,
};

class Thread {
	constructor(
		public data: any,
		public messages: Array<string> = [],
	) {}
}

export function getTimestampFromUUID(uuid: string): Date {
	const bytes = UUID.parse(uuid).bytes;
	const timestamp = bytes.slice(0, 6).reduce(
		(acc: number, e: number) => acc * 256 + e,
		0,
	);
	return new Date(timestamp);
}

function reconcile(base: Data, msg: any): Data {
	if (msg.type === "ready") {
		return { ...base, user: msg.user };
	} else if (msg.type === "upsert.room") {
		return { ...base, rooms: { ...base.rooms, [msg.room.room_id]: msg.room } };
	} else if (msg.type === "upsert.thread") {
		return {
			...base,
			threads: {
				...base.threads,
				[msg.thread.thread_id]: new Thread(msg.thread, []),
			},
		};
	} else if (msg.type === "upsert.message") {
		const { message } = msg;
		const thread = base.threads[message.thread_id];
		return {
			...base,
			threads: {
				...base.threads,
				[message.thread_id]: {
					...thread,
					messages: [
						...thread.messages.filter((i) => i !== message.nonce),
						message.message_id,
					],
				},
			},
			messages: {
				...base.messages,
				[message.message_id]: message,
				[message.nonce]: null,
			},
		};
	} else if (msg.type !== "ping") {
		console.warn("unknown message type", msg.type);
	}
	return base;
}

export class Client {
	public data = blankData;
	private ws: WebSocket | undefined;

	constructor(
		private token: string,
		private baseUrl: string,
		private onReady: () => void,
		private onClose: () => void,
		private onData: (data: Data) => void,
	) {}

	connect() {
		this.ws = new WebSocket(`${this.baseUrl}/api/v1/sync`);

		this.ws.onopen = () => {
			console.log("opened");
			this.ws!.send(JSON.stringify({ type: "hello", token: this.token }));
		};

		this.ws.onclose = () => {
			console.log("closed");
			this.onClose();
		};

		this.ws.onmessage = (ev) => {
			const msg = JSON.parse(ev.data);
			console.log("recv", msg);
			this.handleMessage(msg);
		};
	}

	handleMessage(msg: any) {
		console.log("recv", msg);
		this.data = reconcile(this.data, msg);
		this.onData(this.data);
		if (msg.type === "ping") {
			this.ws!.send(JSON.stringify({ type: "pong" }));
		} else if (msg.type === "ready") {
			this.onReady();
		}
	}

	async http(
		method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
		url: string,
		body?: any,
	) {
		console.log(`${method} ${url}`);
		const req = await fetch(`${this.baseUrl}${url}`, {
			method,
			headers: {
				"authorization": this.token,
				// "content-type": body ? "application/json" : null,
				"content-type": "application/json",
			},
			body: body ? JSON.stringify(body) : undefined,
		});
		if (!req.ok) {
			throw new Error(`request failed (${req.status}): ${await req.text()}`);
		}
		return req.json();
	}
}

// async function test() {
//   const http = createHttp(TOKEN);
//   const room = await http("POST", "/api/v1/rooms", { name: "arst" });
//   await http("GET", "/api/v1/rooms");
//   await http("PATCH", `/api/v1/rooms/${room.room_id}`, { description: "foobar" });
//   await http("GET", `/api/v1/rooms/${room.room_id}`);
//   const thread = await http("POST", `/api/v1/rooms/${room.room_id}/threads`, { name: "test thread" });
//   const message = await http("POST", `/api/v1/rooms/${room.room_id}/threads/${thread.thread_id}/messages`, { content: "hello world" });
//   await http("GET", `/api/v1/rooms/${room.room_id}/threads/${thread.thread_id}/messages`);
//   await http("PATCH", `/api/v1/rooms/${room.room_id}/threads/${thread.thread_id}/messages/${message.message_id}`, { content: "goodbye world" });
//   await http("GET", `/api/v1/rooms/${room.room_id}/threads/${thread.thread_id}/messages`);
// }
