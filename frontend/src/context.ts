import { Accessor, createContext, Setter } from "solid-js";
import { Client } from "sdk";
import { SetStoreFunction } from "solid-js/store";
import { MessageT, RoomT, ThreadT, UserT } from "./types.ts";

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

export type Data = {
	rooms: Record<string, RoomT>,
	threads: Record<string, ThreadT>,
	messages: Record<string, MessageT>,
	timelines: Record<string, Array<TimelineItem>>,
	slices: Record<string, Slice>,
	user: UserT | null,
	menu: any | null,
	view: View,
}

export type Action
	= { do: "setView", to: View }
	| { do: "paginate", thread_id: string, dir: "f" | "b" }
	| { do: "goto", thread_id: string, event_id: string }
	| { do: "menu", menu: any }

export type ChatProps = {
	client: Client;
	data: Data,
	dispatch: (action: Action) => Promise<void>,
};

export const chatctx = createContext<ChatProps>();
