import { Message, Pagination } from "sdk";
import { createContext, useContext } from "solid-js";
import { MessageListAnchor } from "../api/services/MessagesService.ts";
import { Attachment } from "../context";
import { EditorState } from "prosemirror-state";
import { SetStoreFunction, Store } from "solid-js/store";

export type ChannelSearch = {
	query: string;
	results: Pagination<Message> | null;
	loading: boolean;
	author?: string[];
	before?: string;
	after?: string;
	channel?: string[];
};

export type ChannelState = {
	anchor?: MessageListAnchor;
	attachments: Array<Attachment>;
	editor_state?: EditorState;
	highlight?: string;
	read_marker_id?: string;
	reply_id?: string;
	scroll_pos?: number;
	search?: ChannelSearch;
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
	};
}

export type ChannelContextT = [
	Store<ChannelState>,
	SetStoreFunction<ChannelState>,
];

export const ChannelContext = createContext<ChannelContextT>();
export const useChannel = () => useContext(ChannelContext);
