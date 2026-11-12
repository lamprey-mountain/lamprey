import { ReactiveMap } from "@solid-primitives/map";
import type { EditorState } from "prosemirror-state";
import { createContext, type ParentProps, useContext } from "solid-js";

export type MessageDraft = {
	state: EditorState;
	updated_at: number;
};

export type MessageDraftsContextT = {
	get: (channel_id: string) => MessageDraft | undefined;
	set: (channel_id: string, state: EditorState) => void;
};

const MessageDraftsContext = createContext<MessageDraftsContextT>();

export const MessageDraftsProvider = (props: ParentProps) => {
	const drafts = new ReactiveMap<string, MessageDraft>();

	const value: MessageDraftsContextT = {
		get: (channel_id) => {
			// track key existence as well as the value, so a first draft is reactive
			drafts.has(channel_id);
			return drafts.get(channel_id);
		},
		set: (channel_id, state) => {
			drafts.set(channel_id, { state, updated_at: Date.now() });
		},
	};

	return (
		<MessageDraftsContext.Provider value={value}>
			{props.children}
		</MessageDraftsContext.Provider>
	);
};

export const useMessageDrafts = (): MessageDraftsContextT => {
	const ctx = useContext(MessageDraftsContext);
	if (!ctx) {
		throw new Error(
			"useMessageDrafts must be used within a MessageDraftsProvider",
		);
	}
	return ctx;
};
