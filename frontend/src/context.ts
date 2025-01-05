import { Accessor, createContext, Setter } from "solid-js";
import { Client, Room, Thread } from "sdk";
import { SetStoreFunction } from "solid-js/store";
import { MessageT, RoomT, ThreadT, UserT } from "./types.ts";

export type View
	= { view: "home" }
  | { view: "room", room: RoomT }
  | { view: "room-settings", room: RoomT }
  | { view: "thread", thread: ThreadT, room: RoomT }

export type Timeline = {
	messages: Array<MessageT>,
	is_at_beginning: boolean,
	is_at_end: boolean,
	thread_id: string,
}

export type TimelineSet = {
	list: Array<Timeline>
}

export type Data = {
	rooms: Record<string, RoomT>,
	threads: Record<string, ThreadT>,
	messages: Record<string, MessageT>,
	timelines: Record<string, TimelineSet>,
	user: UserT | null,
	view: View,
}

export type Action
	= { do: "setView", to: View }
	| { do: "paginate", thread_id: string, timeline?: Timeline, dir: "f" | "b" }

export type ChatProps = {
	client: Client;
	data: Data,
	dispatch: (action: Action) => Promise<void>,
};

export const chatctx = createContext<ChatProps>();
