import { createContext, useContext } from "solid-js";
import { Client } from "sdk";
import { InviteT, MemberT, MessageT, RoleT, RoomT, ThreadT, UserT } from "./types.ts";
import type { EditorState } from "prosemirror-state";

export type View
	= { view: "home" }
  | { view: "room", room: RoomT }
  | { view: "room-settings", room: RoomT }
  | { view: "thread", thread: ThreadT, room: RoomT }

export type TimelineItem
	= { type: "remote", message: MessageT }
	| { type: "local",  message: MessageT }
	| { type: "hole" }

type Slice = {
	start: number,
	end: number,
}

type InputState = {
	state: EditorState,
	reply_id: string | null,
}

export type Data = {
	rooms: Record<string, RoomT>,
	room_members: Record<string, Record<string, MemberT>>,
	room_roles: Record<string, Record<string, RoleT>>,
	threads: Record<string, ThreadT>,
	messages: Record<string, MessageT>,
	timelines: Record<string, Array<TimelineItem>>,
	slices: Record<string, Slice>,
	invites: Record<string, InviteT>,
	users: Record<string, UserT>,
	user: UserT | null,
	edit_states: Record<string, InputState>,
	modals: Array<any>,
	menu: any | null,
	view: View,
}

type Menu = {
	x: number,
	y: number,
} & (any
| { type: "room", room: RoomT }
| { type: "thread", thread: ThreadT }
| { type: "message", message: MessageT }
)

export type Action
	= { do: "setView", to: View }
	| { do: "paginate", thread_id: string, dir: "f" | "b" }
	| { do: "goto", thread_id: string, event_id: string }
	| { do: "menu", menu: Menu }
	// | { do: "modal.open", modal: any }
	| { do: "modal.close" }
	| { do: "modal.prompt", text: string }
	| { do: "modal.alert", text: string }
	| { do: "modal.confirm", text: string }
	| { do: "editor.init", thread_id: string }
	| { do: "editor.reply", thread_id: string, reply_id: string | null }

export type ChatCtx = {
	client: Client;
	data: Data,
	dispatch: (action: Action) => Promise<any>,
};

export const chatctx = createContext<ChatCtx>();
export const useCtx = () => useContext(chatctx)!;
