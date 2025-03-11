import {
	type Accessor,
	createContext,
	type Setter,
	useContext,
} from "solid-js";
import type { Client, Media, MessageReady, MessageSync, Upload } from "sdk";
import type { EditorState } from "prosemirror-state";
import type { MessageListAnchor } from "./api/messages.ts";
import type { ReactiveMap } from "@solid-primitives/map";
import type { Emitter } from "@solid-primitives/event-bus";
import type * as i18n from "@solid-primitives/i18n";
import type en from "./i18n/en.ts";

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
		| { type: "thread"; thread_id: string }
		| {
			type: "message";
			thread_id: string;
			message_id: string;
			version_id: string;
		}
		| { type: "member_room"; user_id: string; room_id: string }
		| { type: "member_thread"; user_id: string; thread_id: string }
		| { type: "user"; user_id: string }
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
	};

export type Action =
	| { do: "paginate"; thread_id: string; dir: "f" | "b" }
	| { do: "goto"; thread_id: string; event_id: string }
	| { do: "menu.preview"; id: string | null }
	| { do: "modal.open"; modal: Modal }
	| { do: "modal.close" }
	| { do: "modal.alert"; text: string }
	| { do: "modal.prompt"; text: string; cont: (text: string | null) => void }
	| { do: "modal.confirm"; text: string; cont: (confirmed: boolean) => void }
	| { do: "thread.send"; thread_id: string; text: string }
	| {
		do: "thread.scroll_pos";
		thread_id: string;
		pos: number | null;
		is_at_end: boolean;
	}
	| {
		do: "thread.mark_read";
		thread_id: string;
		message_id?: string;
		version_id: string;
		delay?: boolean;
		also_local?: boolean;
	}
	| { do: "upload.init"; local_id: string; thread_id: string; file: File }
	| { do: "upload.pause"; local_id: string }
	| { do: "upload.resume"; local_id: string }
	| { do: "upload.cancel"; local_id: string; thread_id: string }
	| { do: "server.init_session" }
	| { do: "window.mouse_move"; e: MouseEvent };

export type AttachmentCreateT = {
	id: string;
};

export type ChatCtx = {
	client: Client;
	data: Data;
	dispatch: (action: Action) => void;

	t: i18n.NullableTranslator<i18n.Flatten<typeof en>>;
	events: Emitter<Events>;
	menu: Accessor<Menu | null>;
	thread_anchor: ReactiveMap<string, MessageListAnchor>;
	thread_attachments: ReactiveMap<string, Array<Attachment>>;
	thread_editor_state: Map<string, EditorState>;
	thread_highlight: Map<string, string>;
	thread_read_marker_id: ReactiveMap<string, string>;
	thread_reply_id: ReactiveMap<string, string>;
	thread_scroll_pos: Map<string, number>;
	uploads: ReactiveMap<string, Upload>;

	currentMedia: Accessor<MediaCtx | null>;
	setCurrentMedia: Setter<MediaCtx | null>;
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
