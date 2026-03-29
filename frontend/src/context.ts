import {
	type Accessor,
	createContext,
	type Setter,
	useContext,
} from "solid-js";
import type {
	Client,
	Media,
	MessageReady,
	MessageSync,
	Preferences,
	Upload,
} from "sdk";
import type { ReactiveMap } from "@solid-primitives/map";
import type { Emitter } from "@solid-primitives/event-bus";
import type * as i18n from "@solid-primitives/i18n";
import type en from "./i18n/en.tsx";
import type { Placement } from "@floating-ui/dom";
import type { SlashCommands } from "./contexts/slash-commands";

export type Slice = {
	start: number;
	end: number;
};

export type Attachment =
	& { local_id: string }
	& (
		| {
			status: "uploading";
			file: File;
			progress: number;
			paused: boolean;
			filename?: string;
			alt?: string;
			spoiler?: boolean;
		}
		| {
			status: "uploaded";
			media: Media;
			spoiler?: boolean;
		}
	);

export type Data = {
	cursor: Cursor;
	channels: { [channel_id: string]: { read_marker_id?: string } };
};

export type Cursor = {
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
};

import type { ChannelContextT } from "./channelctx";
import type { RoomContextT } from "./contexts/room.tsx";
import type { DocumentContextT } from "./contexts/document.tsx";
import type { SetStoreFunction } from "solid-js/store";

// TODO: split apart this massive context into more granular contexts
export type ChatCtx = {
	client: Client;
	data: Data;
	dataUpdate: SetStoreFunction<Data>;

	t: i18n.Translator<i18n.Flatten<typeof en>>;
	events: Emitter<Events>;
	popout: Accessor<Popout | null>;
	setPopout: Setter<Popout | null>;

	threadsView: Accessor<ThreadsViewData | null>;
	setThreadsView: Setter<ThreadsViewData | null>;
	uploads: ReactiveMap<string, Upload>; // TODO: verify this is unused then remove
	recentChannels: Accessor<Array<string>>;
	setRecentChannels: Setter<Array<string>>;
	currentMedia: Accessor<MediaCtx | null>;
	setCurrentMedia: Setter<MediaCtx | null>;
	preferences: Accessor<Preferences>;
	setPreferences: Setter<Preferences>;
	scrollToChatList: (pos: number) => void;
	cursorStats: Accessor<CursorStats | null>;
	setCursorStats: Setter<CursorStats | null>;
	slashCommands: SlashCommands;
	channel_contexts: ReactiveMap<string, ChannelContextT>;
	room_contexts: ReactiveMap<string, RoomContextT>;
	document_contexts: ReactiveMap<string, DocumentContextT>;
};

export type CursorStats = {
	x: number;
	y: number;
	label: string | null;
};

export type MediaCtx = {
	media: Media;
	element: HTMLMediaElement;
};

export type Events = {
	sync: MessageSync;
	ready: MessageReady;
};

export type { Menu } from "./contexts/menu.tsx";
export type { Modal } from "./contexts/modal.tsx";

export const chatctx = createContext<ChatCtx>();
export const useCtx = () => useContext(chatctx)!;
