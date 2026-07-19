import type { EditorState } from "prosemirror-state";
import type { Message, Pagination } from "sdk";
import { createContext, useContext } from "solid-js";
import type { SetStoreFunction, Store } from "solid-js/store";
import {
	createTimelineController,
	type TimelineController,
	type TimelineState,
} from "@/components/features/chat/timeline-context";
import type { Attachment } from "@/types/chat";

export type ChannelSearch = {
	query: string;
	results: Pagination<Message> | null;
	loading: boolean;
	author?: string[];
	before?: string;
	after?: string;
	channel?: string[];
	sort?: "newest" | "oldest" | "relevancy";
};

// TODO: use this (see below)
// export type ChannelSidebar
// 	= { type: "pinned_messages" }
// 	| { type: "document_history" }
// 	| { type: "message_search" }
// 	| { type: "voice_chat" } //
// 	| { type: "thread_chat", thread_id: string }
// NOTE: maybe i should make this "layerable" (or an object), since you can eg. open pinned messages inside a voice chat and closing pinned messages should return to the voice chat

// TODO: split this context apart
export type ChannelState = {
	attachments: Array<Attachment>;
	editor_state?: EditorState;
	reply_id?: string;
	search?: ChannelSearch;
	timeline: TimelineController;
	timelineState?: TimelineState;

	// TODO: merge these into sidebar: Sidebar
	pinned_view: boolean;
	voice_chat_sidebar_open: boolean;
	history_view: boolean;
	thread_chat_sidebar_thread_id?: string;

	slowmode_expire_at: Date | null;
	editingMessage?: {
		message_id: string;
		selection?: "start" | "end";
		editor_state: EditorState;
	};
	selectMode: boolean;
	selectedMessages: Array<string>;
	input_focus?: () => void;
	reply_jump_source?: string;
	editing_name?: string | null;
	script_id?: string;
};

export function createInitialChannelState(): ChannelState {
	return {
		attachments: [],
		pinned_view: false,
		voice_chat_sidebar_open: false,
		history_view: false,
		thread_chat_sidebar_thread_id: undefined,
		slowmode_expire_at: null,
		selectMode: false,
		selectedMessages: [],
		timeline: createTimelineController(),
	};
}

export type ChannelContextT = [
	Store<ChannelState>,
	SetStoreFunction<ChannelState>,
];

export const ChannelContext = createContext<ChannelContextT>();
export const useChannel = (): ChannelContextT => {
	const ctx = useContext(ChannelContext);
	if (!ctx) {
		throw new Error("useChannel must be used within a ChannelContext.Provider");
	}
	return ctx;
};

export const useOptionalChannel = (): ChannelContextT | [null, null] => {
	return useContext(ChannelContext) ?? [null, null];
};
