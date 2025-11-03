import {
	type Accessor,
	createContext,
	type Setter,
	useContext,
} from "solid-js";
import type {
	Client,
	Media,
	Message,
	MessageReady,
	MessageSync,
	Pagination,
	Upload,
	UserConfig,
} from "sdk";
import type { Action } from "./dispatch/types";
import type { EditorState } from "prosemirror-state";
import type { MessageListAnchor } from "./api/messages.ts";
import type { ReactiveMap } from "@solid-primitives/map";
import type { Emitter } from "@solid-primitives/event-bus";
import type * as i18n from "@solid-primitives/i18n";
import type en from "./i18n/en.ts";
import { Placement, ReferenceElement } from "@floating-ui/dom";

export type Slice = {
	start: number;
	end: number;
};

export type Attachment =
	& { file: File; local_id: string }
	& (
		| { status: "uploading"; progress: number; paused: boolean }
		| { status: "uploaded"; media: Media }
	);

export type Data = {
	modals: Array<Modal>;
	cursor: Cursor;
};

export type Cursor = {
	preview: string | null;
	vel: number;
	pos: Array<[number, number]>;
};

export type Menu =
	& {
		x: number;
		y: number;
	}
	& (
		| { type: "room"; room_id: string }
		| { type: "channel"; channel_id: string }
		| {
			type: "message";
			channel_id: string;
			message_id: string;
			version_id: string;
		}
		| {
			type: "user";
			user_id: string;
			channel_id?: string;
			room_id?: string;
			admin: boolean;
		}
	);

export type Modal =
	| { type: "alert"; text: string }
	| {
		type: "confirm";
		text: string;
		cont: (confirmed: boolean) => void;
	}
	| {
		type: "prompt";
		text: string;
		cont: (text: string | null) => void;
	}
	| {
		type: "media";
		media: Media;
	}
	| {
		type: "message_edits";
		channel_id: string;
		message_id: string;
	}
	| {
		type: "reset_password";
	}
	| {
		type: "palette";
	};

export type AttachmentCreateT = {
	id: string;
};

export type ChannelSearch = {
	query: string;
	results: Pagination<Message> | null;
	loading: boolean;
	author?: string[];
	before?: string;
	after?: string;
	channel?: string[];
};

export type UserViewData = {
	user_id: string;
	room_id?: string;
	channel_id?: string;
	ref: HTMLElement;
	source?: "member-list" | "message";
};

export type Popout = {
	id?: string;
	ref?: HTMLElement;
	props?: any;
	placement?: Placement;
} | {};

export type AutocompleteState = {
	type: "mention";
	query: string;
	ref: ReferenceElement;
	onSelect: (userId: string, userName: string) => void;
	channelId: string;
} | null;

export type ChatCtx = {
	client: Client;
	data: Data;
	dispatch: (action: Action) => void;

	t: i18n.NullableTranslator<i18n.Flatten<typeof en>>;
	events: Emitter<Events>;
	menu: Accessor<Menu | null>;
	setMenu: Setter<Menu | null>;
	popout: Accessor<Popout>;
	setPopout: Setter<Popout>;
	autocomplete: Accessor<AutocompleteState>;
	setAutocomplete: Setter<AutocompleteState>;
	userView: Accessor<UserViewData | null>;
	setUserView: Setter<UserViewData | null>;
	channel_anchor: ReactiveMap<string, MessageListAnchor>;
	channel_attachments: ReactiveMap<string, Array<Attachment>>;
	channel_editor_state: Map<string, EditorState>;
	channel_highlight: Map<string, string>;
	channel_read_marker_id: ReactiveMap<string, string>;
	channel_reply_id: ReactiveMap<string, string>;
	channel_scroll_pos: Map<string, number>;
	channel_search: ReactiveMap<string, ChannelSearch>;
	channel_pinned_view: ReactiveMap<string, boolean>; // channel_id -> showing_pinned
	voice_chat_sidebar_open: ReactiveMap<string, boolean>;
	uploads: ReactiveMap<string, Upload>;
	channel_edit_drafts: ReactiveMap<string, string>;
	channel_input_focus: Map<string, () => void>;
	channel_slowmode_expire_at: ReactiveMap<string, Date | null>; // channel_id -> expiration time

	editingMessage: ReactiveMap<
		string,
		{ message_id: string; selection?: "start" | "end" }
	>; // channel_id -> message_id

	recentChannels: Accessor<Array<string>>;
	setRecentChannels: Setter<Array<string>>;

	currentMedia: Accessor<MediaCtx | null>;
	setCurrentMedia: Setter<MediaCtx | null>;

	userConfig: Accessor<UserConfig>;
	setUserConfig: Setter<UserConfig>;

	scrollToChatList: (pos: number) => void;

	selectMode: ReactiveMap<string, boolean>; // channel_id -> boolean
	selectedMessages: ReactiveMap<string, string[]>; // channel_id -> message_id[]
};
export type MediaCtx = {
	media: Media;
	element: HTMLMediaElement;
};

export type Events = {
	sync: MessageSync;
	ready: MessageReady;
};

export const chatctx = createContext<ChatCtx>();
export const useCtx = () => useContext(chatctx)!;
