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
import { RolesService } from "../services/RolesService";
import { SessionsService } from "../services/SessionsService";
import { RoomMembersService } from "../services/RoomMembersService";
import { ThreadMembersService } from "../services/ThreadMembersService";
import { MessagesService } from "../services/MessagesService";
import { Emitter } from "@solid-primitives/event-bus";

export class RootStore {
	client: Client;
	rooms: RoomsService;
	channels: ChannelsService;
	users: UsersService;
	roles: RolesService;
	sessions: SessionsService;
	roomMembers: RoomMembersService;
	threadMembers: ThreadMembersService;
	messages: MessagesService;

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
		this.roles = new RolesService(this);
		this.sessions = new SessionsService(this);
		this.roomMembers = new RoomMembersService(this);
		this.threadMembers = new ThreadMembersService(this);
		this.messages = new MessagesService(this);

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
			for (const role of msg.roles) {
				this.roles.upsert(role);
			}
			for (const member of msg.room_members) {
				this.roomMembers.upsert(member);
			}
		} else if (msg.type === "RoomCreate" || msg.type === "RoomUpdate") {
			this.rooms.upsert(msg.room);
		} else if (
			msg.type === "ChannelCreate" || msg.type === "ChannelUpdate"
		) {
			if ("channel" in msg) {
				this.channels.upsert(msg.channel);
			}
		} else if (msg.type === "UserCreate" || msg.type === "UserUpdate") {
			this.users.upsert(msg.user);
		} else if (msg.type === "PresenceUpdate") {
			const { user_id, presence } = msg;
			const user = this.users.get(user_id);
			if (user) {
				this.users.upsert({ ...user, presence });
			}
		} else if (msg.type === "RoleCreate" || msg.type === "RoleUpdate") {
			this.roles.upsert(msg.role);
		} else if (msg.type === "RoleDelete") {
			this.roles.delete(msg.role_id);
		} else if (msg.type === "SessionCreate") {
			const s = this.session();
			if (
				msg.session?.id === s?.id && s?.status === "Unauthorized" &&
				msg.session.status === "Authorized"
			) {
				location.reload();
			}
		} else if (msg.type === "SessionUpdate") {
			if (msg.session?.id === this.session()?.id) {
				this.setSession(msg.session);
			}
		} else if (
			msg.type === "RoomMemberCreate" || msg.type === "RoomMemberUpdate"
		) {
			this.roomMembers.upsert(msg.member);
		} else if (msg.type === "RoomMemberDelete") {
			// RoomMember doesn't have a simple ID to delete, but we can assume we might want to remove it
			// if we were tracking it. However, the cache uses `room_id:user_id`.
			const key = this.roomMembers.getKey(
				msg.room_id as any,
				msg.user_id as any,
			); // hack or use helper
			// Actually BaseService key generator for RoomMember uses item properties.
			// We can construct a fake item or expose getKey helper.
			this.roomMembers.cache.delete(`${msg.room_id}:${msg.user_id}`);
		} else if (msg.type === "ThreadMemberUpsert") {
			for (const member of msg.added) {
				this.threadMembers.upsert(member);
			}
			for (const user_id of msg.removed) {
				this.threadMembers.cache.delete(`${msg.thread_id}:${user_id}`);
			}
		} else if (msg.type === "MessageCreate") {
			// Need to pass to MessagesService to update ranges
			this.messages.upsert(msg.message as any); // Cast because SDK type mismatch?
			// Actually MessagesService expects Message.
			const m = msg.message as any;
			if (raw.op === "Sync") m.nonce = raw.nonce;

			// Check mentions etc (logic from api.tsx) - Skipping for now to focus on data sync

			const ranges = this.messages.cacheRanges.get(m.channel_id);
			if (ranges) {
				if (m.nonce) {
					// Local echo handling - specialized logic inside MessagesService or here?
					// Ideally MessagesService handles this.
				}
				ranges.live.items.push(m);
				this.messages.updateMutators(m.channel_id);
			}
		} else if (msg.type === "MessageUpdate") {
			const m = msg.message as any;
			this.messages.upsert(m);
			const ranges = this.messages.cacheRanges.get(m.channel_id);
			if (ranges) {
				const idx = ranges.live.items.findIndex((i) => i.id === m.id);
				if (idx !== -1) ranges.live.items[idx] = m;
				this.messages.updateMutators(m.channel_id);
			}
		}
		// TODO: MessageDelete, etc.
	}
}
