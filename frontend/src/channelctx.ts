import { ReactiveSet } from "@solid-primitives/set";
import { Message, Pagination } from "sdk";
import { Accessor, createContext, Setter, useContext } from "solid-js";
import { MessageListAnchor } from "./api/messages";
import { Attachment } from "./context";
import { EditorState } from "prosemirror-state";
import { createStore } from "solid-js/store";

export type ChannelSearch = {
	query: string;
	results: Pagination<Message> | null;
	loading: boolean;
	author?: string[];
	before?: string;
	after?: string;
	channel?: string[];
};

export type ChannelContext = {
	anchor: MessageListAnchor;
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
};

export const channelctx = createContext<ChannelContext>();
export const useChannel = () => useContext(channelctx)!;

export const defaultChannelState = (anchor: MessageListAnchor) =>
	createStore<ChannelContext>(
		{
			anchor,
			attachments: [],
			pinned_view: false,
			voice_chat_sidebar_open: false,
			slowmode_expire_at: null,
			selectMode: false,
			selectedMessages: [],
		},
	);
