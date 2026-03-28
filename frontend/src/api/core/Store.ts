import {
	Client,
	Message,
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
import { PreferencesService } from "../services/PreferencesService";
export { MemberListService } from "../services/MemberListService";
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
import { InboxService } from "../services/InboxService";
import { DocumentsService } from "../services/DocumentsService";
import { Emitter } from "@solid-primitives/event-bus";
import type { IDBPDatabase } from "idb";
import { ReactiveMap } from "@solid-primitives/map";
import type { UserWithRelationship, VoiceState } from "sdk";
import { logger } from "../../logger";
import {
	stripMarkdownAndResolveMentions as stripMarkdownAndResolveMentionsOriginal,
} from "../../notification-util.ts";

const storeLog = logger.for("api/rooms");

export class RootStore {
	client: Client;
	private _events: Emitter<{
		sync: [MessageSync, MessageEnvelope];
		ready: MessageReady;
	}>;
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
	inbox: InboxService;
	documents: DocumentsService;
	preferences: PreferencesService;
	voiceStates: ReactiveMap<string, VoiceState>;
	typing: ReactiveMap<string, Set<string>>;

	session: Accessor<Session | null>;
	setSession: (s: Session | null) => void;

	// Backwards compatibility aliases
	get room_members() {
		return this.roomMembers;
	}
	get thread_members() {
		return this.threadMembers;
	}
	get room_bans() {
		return this.roomBans;
	}
	get audit_logs() {
		return this.auditLog;
	}
	get room_analytics() {
		return this.roomAnalytics;
	}
	get voiceState(): VoiceState | null {
		const session = this.session();
		if (!session || session.status === "Unauthorized") return null;
		const userId = session.user_id;
		if (!userId) return null;
		return this.voiceStates.get(userId) ?? null;
	}

	stripMarkdownAndResolveMentions(
		content: string,
		thread_id: string,
		mentions?: Message["latest_version"]["mentions"],
	): Promise<string> {
		return stripMarkdownAndResolveMentionsOriginal(
			content,
			thread_id,
			this,
			mentions,
		);
	}

	// Backwards compatibility - events are accessed via store directly
	get events() {
		return this._events;
	}

	constructor(
		client: Client,
		events: Emitter<{
			sync: [MessageSync, MessageEnvelope];
			ready: MessageReady;
		}>,
		getDb?: () => IDBPDatabase<unknown> | undefined,
	) {
		this.client = client;
		this._events = events;

		const [session, setSession] = createSignal<Session | null>(null);
		this.session = session;
		this.setSession = setSession;

		this.auditLog = new AuditLogService(this, getDb);
		this.auth = new AuthService(this, getDb);
		this.channels = new ChannelsService(this, getDb);
		this.dms = new DmsService(this, getDb);
		this.documents = new DocumentsService(this, getDb);
		this.emoji = new EmojiService(this, getDb);
		this.inbox = new InboxService(this, getDb);
		this.invites = new InvitesService(this, getDb);
		this.memberLists = new MemberListService(this);
		this.messages = new MessagesService(this, getDb);
		this.notifications = new NotificationService(this);
		this.preferences = new PreferencesService(this);
		this.push = new PushService(this, getDb);
		this.reactions = new ReactionsService(this, getDb);
		this.roles = new RolesService(this, getDb);
		this.roomAnalytics = new RoomAnalyticsService(this, getDb);
		this.roomBans = new RoomBansService(this, getDb);
		this.roomMembers = new RoomMembersService(this, getDb);
		this.rooms = new RoomsService(this, getDb);
		this.sessions = new SessionsService(this, getDb);
		this.tags = new TagsService(this, getDb);
		this.threadMembers = new ThreadMembersService(this, getDb);
		this.threads = new ThreadsService(this, getDb);
		this.users = new UsersService(this, getDb);
		this.webhooks = new WebhooksService(this, getDb);

		this.voiceStates = new ReactiveMap();
		this.typing = new ReactiveMap();

		this._events.on("sync", ([msg, raw]) => this.handleSync(msg, raw));
		this._events.on("ready", (msg) => this.handleReady(msg));
	}

	handleReady(msg: MessageReady) {
		storeLog.info("Ready message received", {
			session: msg.session,
			has_user: !!msg.user,
			user_id: msg.user?.id,
		});
		this.setSession(msg.session);
		if (msg.user) {
			// Set @self alias first using session user_id
			const userId = msg.session.status === "Unauthorized"
				? undefined
				: msg.session.user_id;
			storeLog.debug("Setting @self alias", {
				userId,
				session_user_id: msg.session.status === "Unauthorized"
					? undefined
					: msg.session.user_id,
			});
			if (userId) {
				const userWithRelationship: UserWithRelationship = {
					...msg.user,
					relationship: {
						note: null,
						relation: null,
						petname: null,
					},
				};
				this.users.cache.set("@self", userWithRelationship);
				storeLog.info("@self alias set", {
					"@self": this.users.cache.get("@self"),
					cache_size: this.users.cache.size,
				});
			}
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
			if (msg.config) {
				this.preferences.cache.set("@self", msg.config);
				this.preferences._loaded = true;
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
			this.memberLists.updateUser(msg.user);
		} else if (msg.type === "PresenceUpdate") {
			const { user_id, presence } = msg;
			const user = this.users.get(user_id);
			if (user) {
				const updatedUser = { ...user, presence };
				this.users.upsert(updatedUser);
				this.memberLists.updateUser(updatedUser);
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
			const m = msg.message;
			if (raw.op === "Sync" && raw.nonce) {
				(m as any).nonce = raw.nonce;
			}
			this.messages.handleMessageCreate(m);
			this.notifications.handleMessageCreate(m);

			const session = this.session();
			const sessionUserId = session?.status === "Unauthorized"
				? undefined
				: session?.user_id;
			const isOwnMessage = m.author_id === sessionUserId;
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
			this.messages.handleMessageUpdate(msg.message);
		} else if (msg.type === "MessageDelete") {
			this.messages.handleMessageDelete(msg.channel_id, msg.message_id);
		} else if (msg.type === "PreferencesGlobal") {
			const session = this.session();
			const sessionUserId = session && session.status !== "Unauthorized"
				? session.user_id
				: undefined;
			if (msg.user_id === sessionUserId) {
				this.preferences.cache.set("@self", msg.config);
				this.preferences._loaded = true;
			}
		} else if (msg.type === "MemberListSync") {
			this.memberLists.handleSync(msg as any);
		} else if (msg.type === "VoiceState") {
			if (msg.state) {
				this.voiceStates.set(msg.user_id, msg.state);
			} else {
				this.voiceStates.delete(msg.user_id);
			}
		}
	}

	async tempCreateSession() {
		const session = await this.auth.createTempSession();
		if (session.status !== "Unauthorized") {
			const sessionWithToken = session as Session & { token: string };
			localStorage.setItem("token", sessionWithToken.token);
			this.setSession(session);
			this.client.start(sessionWithToken.token);
		}
	}
}
