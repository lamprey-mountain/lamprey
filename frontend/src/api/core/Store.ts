import {
	Client,
	MessageEnvelope,
	MessageReady,
	MessageSync,
	Preferences,
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
import { NotificationService } from "../services/NotificationService";
import { MemberListService } from "../services/MemberListService";
import { InvitesService } from "../services/InvitesService";
import { AuthService } from "../services/AuthService";
import { DmsService } from "../services/DmsService";
import { EmojiService } from "../services/EmojiService";
import { PushService } from "../services/PushService";
import { ReactionsService } from "../services/ReactionsService";
import { RoomAnalyticsService } from "../services/RoomAnalyticsService";
import { RoomBansService } from "../services/RoomBansService";
import { TagsService } from "../services/TagsService";
import { ThreadsService } from "../services/ThreadsService";
import { WebhooksService } from "../services/WebhooksService";
import { AuditLogService } from "../services/AuditLogService";
import { Emitter } from "@solid-primitives/event-bus";
import type { IDBPDatabase } from "idb";

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
	notifications: NotificationService;
	memberLists: MemberListService;
	invites: InvitesService;
	auth: AuthService;
	dms: DmsService;
	emoji: EmojiService;
	push: PushService;
	reactions: ReactionsService;
	roomAnalytics: RoomAnalyticsService;
	roomBans: RoomBansService;
	tags: TagsService;
	threads: ThreadsService;
	webhooks: WebhooksService;
	auditLog: AuditLogService;

	session: Accessor<Session | null>;
	setSession: (s: Session | null) => void;

	preferences?: Accessor<Preferences>;
	setPreferences?: (p: Preferences) => void;
	setServerPreferences?: (p: Preferences) => void;

	constructor(
		client: Client,
		events: Emitter<{
			sync: [MessageSync, MessageEnvelope];
			ready: MessageReady;
		}>,
		preferences?: Accessor<Preferences>,
		setPreferences?: (p: Preferences) => void,
		setServerPreferences?: (p: Preferences) => void,
		getDb?: () => IDBPDatabase<unknown> | undefined,
	) {
		this.client = client;
		this.preferences = preferences;
		this.setPreferences = setPreferences;
		this.setServerPreferences = setServerPreferences;

		const [session, setSession] = createSignal<Session | null>(null);
		this.session = session;
		this.setSession = setSession;

		this.rooms = new RoomsService(this, getDb);
		this.channels = new ChannelsService(this, getDb);
		this.users = new UsersService(this, getDb);
		this.roles = new RolesService(this, getDb);
		this.sessions = new SessionsService(this, getDb);
		this.roomMembers = new RoomMembersService(this, getDb);
		this.threadMembers = new ThreadMembersService(this, getDb);
		this.messages = new MessagesService(this, getDb);
		this.notifications = new NotificationService(this);
		this.memberLists = new MemberListService(this);
		this.invites = new InvitesService(this, getDb);
		this.auth = new AuthService(this, getDb);
		this.dms = new DmsService(this, getDb);
		this.emoji = new EmojiService(this, getDb);
		this.push = new PushService(this, getDb);
		this.reactions = new ReactionsService(this, getDb);
		this.roomAnalytics = new RoomAnalyticsService(this, getDb);
		this.roomBans = new RoomBansService(this, getDb);
		this.tags = new TagsService(this, getDb);
		this.threads = new ThreadsService(this, getDb);
		this.webhooks = new WebhooksService(this, getDb);
		this.auditLog = new AuditLogService(this, getDb);

		events.on("sync", ([msg, raw]) => this.handleSync(msg, raw));
		events.on("ready", (msg) => this.handleReady(msg));
	}

	handleReady(msg: MessageReady) {
		this.setSession(msg.session);
		if (msg.user) {
			this.users.upsert(msg.user);
		}
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
			if (msg.config && this.setPreferences) {
				this.setPreferences(msg.config);
			}
			if (msg.config && this.setServerPreferences) {
				this.setServerPreferences(msg.config);
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
			this.memberLists.updateUser(msg.user as any);
		} else if (msg.type === "PresenceUpdate") {
			const { user_id, presence } = msg;
			const user = this.users.get(user_id);
			if (user) {
				const updatedUser = { ...user, presence };
				this.users.upsert(updatedUser);
				this.memberLists.updateUser(updatedUser as any);
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
			this.memberLists.updateMember(msg.member.user_id, msg.member.room_id);
		} else if (msg.type === "RoomMemberDelete") {
			this.roomMembers.cache.delete(`${msg.room_id}:${msg.user_id}`);
			// TODO: MemberList delete logic
		} else if (msg.type === "ThreadMemberUpsert") {
			for (const member of msg.added) {
				this.threadMembers.upsert(member);
				this.memberLists.updateMember(
					member.user_id,
					undefined,
					member.thread_id,
				);
			}
			for (const user_id of msg.removed) {
				this.threadMembers.cache.delete(`${msg.thread_id}:${user_id}`);
				this.memberLists.updateMember(user_id, undefined, msg.thread_id);
			}
		} else if (msg.type === "MessageCreate") {
			const m = msg.message as any;
			if (raw.op === "Sync") m.nonce = raw.nonce;
			this.messages.handleMessageCreate(m);
			this.notifications.handleMessageCreate(m);

			const session = this.session();
			const isOwnMessage = m.author_id === session?.user_id;
			if (isOwnMessage) {
				const channel = this.channels.cache.get(m.channel_id);
				if (channel) {
					this.channels.cache.set(m.channel_id, {
						...channel,
						message_count: (channel.message_count ?? 0) + 1,
						mention_count: 0,
						last_version_id: m.latest_version.version_id,
						last_read_id: m.latest_version.version_id,
						is_unread: false,
					});
				}
			}
		} else if (msg.type === "MessageUpdate") {
			this.messages.handleMessageUpdate(msg.message as any);
		} else if (msg.type === "MessageDelete") {
			this.messages.handleMessageDelete(msg.channel_id, msg.message_id);
		} else if (msg.type === "PreferencesGlobal") {
			if (
				msg.user_id === (this.session() as any)?.user_id && this.setPreferences
			) {
				this.setPreferences(msg.config);
				if (this.setServerPreferences) {
					this.setServerPreferences(msg.config);
				}
			}
		} else if (msg.type === "MemberListSync") {
			this.memberLists.handleSync(msg);
		}
	}
}
