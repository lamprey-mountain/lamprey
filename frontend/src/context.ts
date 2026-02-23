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
import type { ReactiveMap } from "@solid-primitives/map";
import type { Emitter } from "@solid-primitives/event-bus";
import type * as i18n from "@solid-primitives/i18n";
import type en from "./i18n/en.tsx";
import { Placement, ReferenceElement } from "@floating-ui/dom";
import { SlashCommands } from "./contexts/slash-commands";
import type { Modal } from "./contexts/modal";

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
	cursor: Cursor;
};

export type Cursor = {
	preview: string | null;
	vel: number;
	pos: Array<[number, number]>;
};

export type AttachmentCreateT = {
	id: string;
};

export type ChannelSearch = {
	query: string;
	results: import("sdk").MessageSearch | null;
	loading: boolean;
	author?: string[];
	before?: string;
	after?: string;
	channel?: string[];
};

export type ThreadsViewData = {
	channel_id: string;
	ref: HTMLElement;
};

export type Popout = {
	id?: string;
	ref?: HTMLElement;
	props?: any;
	placement?: Placement;
} | {};

import type { ChannelContextT } from "./channelctx";
import type { RoomContextT } from "./contexts/room.tsx";
import { DocumentContextT } from "./contexts/document.tsx";

// TODO: split apart this massive context into more granular contexts
export type ChatCtx = {
	client: Client;
	data: Data;
	dispatch: (action: Action) => void;

	t: i18n.NullableTranslator<i18n.Flatten<typeof en>>;
	events: Emitter<Events>;
	popout: Accessor<Popout>;
	setPopout: Setter<Popout>;

	threadsView: Accessor<ThreadsViewData | null>;
	setThreadsView: Setter<ThreadsViewData | null>;
	uploads: ReactiveMap<string, Upload>; // TODO: verify this is unused then remove
	recentChannels: Accessor<Array<string>>;
	setRecentChannels: Setter<Array<string>>;
	currentMedia: Accessor<MediaCtx | null>;
	setCurrentMedia: Setter<MediaCtx | null>;
	userConfig: Accessor<UserConfig>;
	setUserConfig: Setter<UserConfig>;
	scrollToChatList: (pos: number) => void;
	slashCommands: SlashCommands;
	channel_contexts: ReactiveMap<string, ChannelContextT>;
	room_contexts: ReactiveMap<string, RoomContextT>;
	document_contexts: ReactiveMap<string, DocumentContextT>;
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
