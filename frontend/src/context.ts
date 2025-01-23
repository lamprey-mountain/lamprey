import { createContext, useContext } from "solid-js";
import { Client, types, Upload } from "sdk";
import {
	InviteT,
	MediaT,
	MemberT,
	MessageT,
	RoleT,
	RoomT,
	ThreadT,
	UserT,
} from "./types.ts";
import type { EditorState } from "prosemirror-state";
import { TimelineItemT } from "./Messages.tsx";

export type TimelineItem =
	| { type: "remote"; message: MessageT }
	| { type: "local"; message: MessageT }
	| { type: "hole" };

export type Slice = {
	start: number;
	end: number;
};

export type Attachment =
	& { file: File; local_id: string }
	& (
		| { status: "uploading"; progress: number; paused: boolean }
		| { status: "uploaded"; media: MediaT }
	);

export type ThreadState = {
	editor_state: EditorState;
	reply_id: string | null;
	scroll_pos: number | null;
	is_at_end: boolean;
	read_marker_id: string | null;
	attachments: Array<Attachment>;
	timeline: Array<TimelineItemT>;
};

// TODO: use maps instead of records? they might not play as nicely with solidjs, but are nicer overall (and possibly a lil more performant)
export type Data = {
	rooms: Record<string, RoomT>;
	room_members: Record<string, Record<string, MemberT>>;
	room_roles: Record<string, Record<string, RoleT>>;
	threads: Record<string, ThreadT>;
	messages: Record<string, MessageT>;
	timelines: Record<string, Array<TimelineItem>>;
	slices: Record<string, Slice>;
	invites: Record<string, InviteT>;
	users: Record<string, UserT>;
	user: UserT | null;
	thread_state: Record<string, ThreadState>;
	modals: Array<Modal>;
	menu: Menu | null;
	// TODO: remove thread_id requirement
	uploads: Record<string, { up: Upload, thread_id: string }>;
};

type Menu =
	& {
		x: number;
		y: number;
	}
	& (
		| { type: "room"; room: RoomT }
		| { type: "thread"; thread: ThreadT }
		| { type: "message"; message: MessageT }
	);

type Modal =
	| { type: "alert"; text: string }
	| {
		type: "confirm";
		text: string;
		cont: (confirmed: boolean) => void;
	}
	| {
		type: "prompt";
		text: string;
		cont: (text?: string) => void;
	};

export type Action =
	| { do: "paginate"; thread_id: string; dir: "f" | "b" }
	| { do: "goto"; thread_id: string; event_id: string }
	| { do: "menu"; menu: Menu | null }
	// | { do: "modal.open", modal: any }
	| { do: "modal.close" }
	| { do: "modal.alert"; text: string }
	| { do: "modal.prompt"; text: string; cont: (text?: string) => void }
	| { do: "modal.confirm"; text: string; cont: (confirmed: boolean) => void }
	| { do: "thread.init"; thread_id: string; read_id?: string }
	| { do: "thread.reply"; thread_id: string; reply_id: string | null }
	| {
		do: "thread.scroll_pos";
		thread_id: string;
		pos: number | null;
		is_at_end: boolean;
	}
	| { do: "thread.autoscroll"; thread_id: string }
	| {
		do: "thread.mark_read";
		thread_id: string;
		version_id?: string;
		delay?: boolean;
		also_local?: boolean;
	}
	| {
		do: "thread.attachments";
		thread_id: string;
		attachments: Array<Attachment>;
	}
	| { do: "fetch.room"; room_id: string }
	| { do: "fetch.thread"; thread_id: string }
	| { do: "fetch.room_threads"; room_id: string }
	| { do: "upload.init"; local_id: string; thread_id: string; file: File }
	| { do: "upload.pause"; local_id: string }
	| { do: "upload.resume"; local_id: string }
	| { do: "upload.cancel"; local_id: string }
	| { do: "server"; msg: types.MessageServer };

export type AttachmentCreateT = {
	id: string;
};

export type ChatCtx = {
	client: Client;
	data: Data;
	dispatch: (action: Action) => Promise<void>;
};

export const chatctx = createContext<ChatCtx>();
export const useCtx = () => useContext(chatctx)!;
