import type {
	Channel,
	Client,
	MemberListGroup,
	MemberListOp,
	Message,
	MessageEnvelope,
	MessageReady,
	MessageSync,
	Session,
} from "sdk";
import { type Accessor, batch, createSignal } from "solid-js";
import { ChannelsService } from "../services/ChannelsService";
import { MessagesService } from "../services/MessagesService";
import { NotificationService } from "../services/NotificationService";
import {
	DEFAULT_PREFERENCES,
	PreferencesService,
} from "../services/PreferencesService";
import { RolesService } from "../services/RolesService";
import { RoomMembersService } from "../services/RoomMembersService";
import { RoomsService } from "../services/RoomsService";
import { SessionsService } from "../services/SessionsService";
import { ThreadMembersService } from "../services/ThreadMembersService";
import { UsersService } from "../services/UsersService";

export { MemberListService } from "../services/MemberListService";

import type { Emitter } from "@solid-primitives/event-bus";
import { ReactiveMap } from "@solid-primitives/map";
import type { IDBPDatabase } from "idb";
import type { UserWithRelationship, VoiceState } from "sdk";
import { stripMarkdownAndResolveMentions as stripMarkdownAndResolveMentionsOriginal } from "@/lib/notifications/util";
import { type ApiDB, clearApiDatabase } from "@/lib/sync/db";
import { logger } from "@/utils/logger";
import { AuditLogService } from "../services/AuditLogService";
import { AuthService } from "../services/AuthService";
import { DmsService } from "../services/DmsService";
import { DocumentBranchService } from "../services/DocumentBranchService";
import { DocumentsService } from "../services/DocumentsService";
import { DocumentTagService } from "../services/DocumentTagService";
import { EmojiService } from "../services/EmojiService";
import { FlumeService } from "../services/FlumeService";
import { InboxService } from "../services/InboxService";
import { InvitesService } from "../services/InvitesService";
import { MediaService } from "../services/MediaService";
import { MemberListService } from "../services/MemberListService";
import { PushService } from "../services/PushService";
import { ReactionsService } from "../services/ReactionsService";
import { RoomAnalyticsService } from "../services/RoomAnalyticsService";
import { RoomBansService } from "../services/RoomBansService";
import { TagsService } from "../services/TagsService";
import { ThreadsService } from "../services/ThreadsService";
import { WebhooksService } from "../services/WebhooksService";
import { BaseService } from "./Service";

const storeLog = logger.for("api/rooms");

type MemberListSyncMessage = {
	type: "MemberListSync";
	user_id: string;
	room_id?: string | null;
	channel_id?: string | null;
	ops: MemberListOp[];
	groups: MemberListGroup[];
};

export class RootStore {
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
	media: MediaService;
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
	documentBranches: DocumentBranchService;
	documentTags: DocumentTagService;
	preferences: PreferencesService;
	flumes: FlumeService;
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
		public client: Client,
		events: Emitter<{
			sync: [MessageSync, MessageEnvelope];
			ready: MessageReady;
		}>,
		private getDb?: () => IDBPDatabase<ApiDB> | undefined,
	) {
		this._events = events;

		const [session, setSession] = createSignal<Session | null>(null);
		this.session = session;
		this.setSession = setSession;

		this.auditLog = new AuditLogService(this, getDb);
		this.auth = new AuthService(this, getDb);
		this.media = new MediaService(this, getDb);
		this.channels = new ChannelsService(this, getDb);
		this.dms = new DmsService(this, getDb);
		this.documents = new DocumentsService(this, getDb);
		this.documentBranches = new DocumentBranchService(this, getDb);
		this.documentTags = new DocumentTagService(this, getDb);
		this.emoji = new EmojiService(this, getDb);
		this.flumes = new FlumeService(this);
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
			const userId =
				msg.session.status === "Unauthorized" ? undefined : msg.session.user_id;
			storeLog.debug("Setting @self alias", {
				userId,
				session_user_id:
					msg.session.status === "Unauthorized"
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
			// Process resources in chunks to avoid blocking the main thread
			const process = async () => {
				const CHUNK_SIZE = 500;

				const chunkedUpsert = async <T>(
					items: T[],
					upsert: (items: T[]) => void,
				) => {
					for (let i = 0; i < items.length; i += CHUNK_SIZE) {
						upsert(items.slice(i, i + CHUNK_SIZE));
						if (i + CHUNK_SIZE < items.length) {
							await new Promise((r) => setTimeout(r, 0));
						}
					}
				};

				await chunkedUpsert(msg.rooms, (items) => this.rooms.upsertBulk(items));
				await new Promise((r) => setTimeout(r, 0));

				await chunkedUpsert(msg.channels, (items) =>
					this.channels.upsertBulk(items),
				);
				await new Promise((r) => setTimeout(r, 0));

				await chunkedUpsert(msg.threads, (items) =>
					this.channels.upsertBulk(items),
				);
				await new Promise((r) => setTimeout(r, 0));

				await chunkedUpsert(msg.roles, (items) => this.roles.upsertBulk(items));
				await new Promise((r) => setTimeout(r, 0));

				await chunkedUpsert(msg.room_members, (items) =>
					this.roomMembers.upsertBulk(items),
				);

				if (msg.config) {
					batch(() => {
						this.preferences.cache.set("@self", msg.config);
						this.preferences._loaded = true;
					});
				}
			};
			process();
		} else if (msg.type === "RoomCreate" || msg.type === "RoomUpdate") {
			this.rooms.upsert(msg.room);
		} else if (msg.type === "RoomDelete") {
			this.rooms.delete(msg.room_id);
		} else if (msg.type === "ChannelCreate" || msg.type === "ChannelUpdate") {
			if ("channel" in msg) {
				this.channels.upsert(msg.channel);

				const channel = msg.channel as Channel & { parent_id?: string };
				if (channel.parent_id) {
					const messageRanges = this.messages._ranges.get(channel.parent_id);
					if (messageRanges) {
						const range = messageRanges.find(channel.id);
						if (range) {
							for (const message of range.items) {
								if (message.id === channel.id && !message.thread) {
									const updatedMessage = {
										...message,
										thread: channel,
									};
									this.messages.handleMessageUpdate(updatedMessage);
								}
							}
						}
					}
				}
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
				msg.session?.id === s?.id &&
				s?.status === "Unauthorized" &&
				msg.session.status === "Authorized"
			) {
				// HACK: reconnect after auth for Ready/Ambient message
				// TODO: send an Ambient message (with current user) on login
				const token = this.client.opts.token!;
				this.client.stopAggressive();
				this.client.start(token);
			}
		} else if (msg.type === "SessionUpdate") {
			if (msg.session?.id === this.session()?.id) {
				this.setSession(msg.session);
			}
		} else if (msg.type === "SessionDelete") {
			if (msg.id === this.session()?.id) {
				this.handleLogout();
			}
		} else if (msg.type === "SessionDeleteAll") {
			this.handleLogout();
		} else if (
			msg.type === "RoomMemberCreate" ||
			msg.type === "RoomMemberUpdate"
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
				(m as typeof m & { nonce?: string }).nonce = raw.nonce;
			}
			this.messages.handleMessageCreate(m);
			this.notifications.handleMessageCreate(m);
			if (m.flume?.state === "Live") {
				this.flumes.handleCreate(m.channel_id, m);
			}

			const session = this.session();
			const sessionUserId =
				session?.status === "Unauthorized" ? undefined : session?.user_id;
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
			const message = msg.message;
			if (message.flume?.state !== "Live") {
				this.flumes.handleCommit(message);
			}
			this.messages.handleMessageUpdate(message);
		} else if (msg.type === "MessageDelete") {
			this.flumes.handleDelete(msg.message_id);
			this.messages.handleMessageDelete(msg.channel_id, msg.message_id);
		} else if (msg.type === "PreferencesGlobal") {
			const session = this.session();
			const sessionUserId =
				session && session.status !== "Unauthorized"
					? session.user_id
					: undefined;
			if (msg.user_id === sessionUserId) {
				this.preferences.cache.set("@self", msg.config);
				this.preferences._loaded = true;
			}
		} else if (msg.type === "MemberListSync") {
			this.memberLists.handleSync(msg as MemberListSyncMessage);
		} else if (msg.type === "FlumeDelta") {
			this.flumes.handleApply(msg.channel_id, msg.message_id, msg.delta);
		} else if (msg.type === "VoiceState") {
			if (msg.state) {
				this.voiceStates.set(msg.user_id, msg.state);
			} else {
				this.voiceStates.delete(msg.user_id);
			}
		} else if (msg.type === "DocumentTagCreate") {
			this.documentTags.upsert(msg.tag);
		} else if (msg.type === "DocumentTagUpdate") {
			this.documentTags.upsert(msg.tag);
		} else if (msg.type === "DocumentTagDelete") {
			this.documentTags.delete(msg.tag_id);
		} else if (msg.type === "DocumentBranchCreate") {
			this.documentBranches.upsert(msg.branch);
		} else if (msg.type === "DocumentBranchUpdate") {
			this.documentBranches.upsert(msg.branch);
		} else if (msg.type === "DocumentBranchDelete") {
			this.documentBranches.delete(msg.branch_id);
		} else if (msg.type === "MediaProcessed") {
			this.media.upsert(msg.media);
		} else if (msg.type === "MediaUpdate") {
			this.media.upsert(msg.media);
		}
	}

	async initSession() {
		const session = await this.auth.createSession();
		const sessionWithToken = session as Session & { token: string };
		localStorage.setItem("token", sessionWithToken.token);
		this.setSession(session);
		this.client.start(sessionWithToken.token);
	}

	async logout() {
		// delete this session, handleLogout will be called once SessionDelete event is received
		await this.sessions.deleteSession("@self");
	}

	private async handleLogout() {
		// shut down connection
		this.setSession(null);
		this.client.stopAggressive();

		// clear cached data
		batch(() => {
			for (const prop of Object.values(this)) {
				if (prop instanceof BaseService) {
					prop.clear();
				}
			}
		});

		// clear non-BaseService state
		this.memberLists.clear();
		this.voiceStates.clear();
		for (const s of this.typing.values()) {
			s.clear();
		}
		this.typing.clear();

		// clear indexeddb caches
		const db = this.getDb?.();
		if (db) {
			try {
				await clearApiDatabase(db);
				storeLog.info("IndexedDB cleared successfully");
			} catch (e) {
				storeLog.error("Failed to clear IndexedDB", e);
			}
		}

		// clear preferences from localStorage
		localStorage.removeItem("token");
		this.preferences.cache.set("@self", DEFAULT_PREFERENCES);
		this.preferences._loaded = false;

		// create a new session
		this.initSession().catch((err) => {
			logger.for("auth").error("Failed to create temp session", err);
			alert("oh no :(\nsomething went VERY wrong");
		});
	}
}
