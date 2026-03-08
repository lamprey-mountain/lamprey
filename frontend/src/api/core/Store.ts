import {
	Client,
	MessageEnvelope,
	MessageReady,
	MessageSync,
	Session,
    Preferences,
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
    notifications: NotificationService;
    memberLists: MemberListService;

	session: Accessor<Session | null>;
	setSession: (s: Session | null) => void;
    
    preferences?: Accessor<Preferences>;
    setPreferences?: (p: Preferences) => void;

	constructor(
		client: Client,
		events: Emitter<{
			sync: [MessageSync, MessageEnvelope];
			ready: MessageReady;
		}>,
        preferences?: Accessor<Preferences>,
        setPreferences?: (p: Preferences) => void
	) {
		this.client = client;
        this.preferences = preferences;
        this.setPreferences = setPreferences;
        
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
        this.notifications = new NotificationService(this);
        this.memberLists = new MemberListService(this);

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
		} else if (msg.type === "RoomMemberCreate" || msg.type === "RoomMemberUpdate") {
            this.roomMembers.upsert(msg.member);
            this.memberLists.updateMember(msg.member.user_id, msg.member.room_id);
        } else if (msg.type === "RoomMemberDelete") {
            this.roomMembers.cache.delete(`${msg.room_id}:${msg.user_id}`);
            // TODO: MemberList delete logic
        } else if (msg.type === "ThreadMemberUpsert") {
            for (const member of msg.added) {
                this.threadMembers.upsert(member);
                this.memberLists.updateMember(member.user_id, undefined, member.thread_id);
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
        } else if (msg.type === "MessageUpdate") {
            this.messages.handleMessageUpdate(msg.message as any);
        } else if (msg.type === "MessageDelete") {
            this.messages.handleMessageDelete(msg.channel_id, msg.message_id);
        } else if (msg.type === "PreferencesGlobal") {
            if (msg.user_id === this.session()?.user_id && this.setPreferences) {
                this.setPreferences(msg.config);
            }
        } else if (msg.type === "MemberListSync") {
            this.memberLists.handleSync(msg);
        }
	}
}
