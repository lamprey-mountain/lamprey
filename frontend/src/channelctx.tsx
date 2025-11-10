import { Message, Pagination } from "sdk";
import { createContext, useContext } from "solid-js";
import { MessageListAnchor } from "./api/messages";
import { Attachment } from "./context";
import { EditorState } from "prosemirror-state";
import { createStore, SetStoreFunction, Store } from "solid-js/store";
import { useCtx } from "./context";
import { ReactiveMap } from "@solid-primitives/map";

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
	slowmode_expire_at: Date | null;
	editingMessage?: {
		message_id: string;
		selection?: "start" | "end";
		editor_state: EditorState;
	};
	selectMode: boolean;
	selectedMessages: Array<string>;
	edit_draft?: string;
	input_focus?: () => void;
};

export function createInitialChannelState(): ChannelState {
	return {
		attachments: [],
		pinned_view: false,
		voice_chat_sidebar_open: false,
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
