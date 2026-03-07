import {
	Client,
	MessageEnvelope,
	MessageReady,
	MessageSync,
	Session,
} from "sdk";
import { Accessor, createSignal } from "solid-js";
import { RoomsService } from "../services/RoomsService";
import { ChannelsService } from "../services/ChannelsService";
import { UsersService } from "../services/UsersService";
import { Emitter } from "@solid-primitives/event-bus";

export class RootStore {
	client: Client;
	rooms: RoomsService;
	channels: ChannelsService;
	users: UsersService;

	session: Accessor<Session | null>;
	setSession: (s: Session | null) => void;

	constructor(
		client: Client,
		events: Emitter<{
			sync: [MessageSync, MessageEnvelope];
			ready: MessageReady;
		}>,
	) {
		this.client = client;
		const [session, setSession] = createSignal<Session | null>(null);
		this.session = session;
		this.setSession = setSession;

		this.rooms = new RoomsService(this);
		this.channels = new ChannelsService(this);
		this.users = new UsersService(this);

		events.on("sync", ([msg, raw]) => this.handleSync(msg, raw));
		events.on("ready", (msg) => this.handleReady(msg));
	}

	handleReady(msg: MessageReady) {
		if (msg.user) {
			this.users.upsert(msg.user);
		}
		this.setSession(msg.session);
	}

	handleSync(msg: MessageSync, raw: MessageEnvelope) {
		if (msg.type === "Ambient") {
			for (const room of msg.rooms) {
				this.rooms.upsert(room);
			}
			for (const channel of msg.channels) {
				this.channels.upsert(channel);
			}
			for (const thread of msg.threads) {
				this.channels.upsert(thread);
			}
			// TODO: Handle users if they are in ambient? usually they are not, only self?
			// Ambient doesn't seem to have users list based on api.tsx
		} else if (msg.type === "RoomCreate" || msg.type === "RoomUpdate") {
			this.rooms.upsert(msg.room);
		} else if (
			msg.type === "ChannelCreate" || msg.type === "ChannelUpdate"
		) {
			this.channels.upsert(msg.channel); // or msg.thread? type says 'channel' for ChannelUpdate?
			// api.tsx says: const { channel: thread } = msg; for ChannelUpdate.
			// Wait, check api.tsx logic.
			// if (msg.type === "ChannelUpdate") { const { channel: thread } = msg; ... }
			// if (msg.type === "ChannelCreate") { const { channel } = msg; ... }
			// SDK types might be unified.
			if ("channel" in msg) {
				this.channels.upsert(msg.channel);
			}
		} else if (msg.type === "UserCreate" || msg.type === "UserUpdate") {
			this.users.upsert(msg.user);
		} else if (msg.type === "PresenceUpdate") {
			// Logic from api.tsx
			const { user_id, presence } = msg;
			const user = this.users.get(user_id);
			if (user) {
				this.users.upsert({ ...user, presence });
			}
		}
	}
}
