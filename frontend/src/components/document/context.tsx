import {
	createGlobalEmitter,
	type GlobalEmitter,
} from "@solid-primitives/event-bus";
import type { EditorState } from "prosemirror-state";
import { createContext, type ParentProps, useContext } from "solid-js";
import type * as Y from "yjs";
import type { ChannelT } from "@/types";
import type { ChangesetSelection } from "./types";

export type DocumentController = {
	events: GlobalEmitter<DocumentEvents>;
	commands: GlobalEmitter<DocumentCommands>;
};

export type DocumentEvents = {
	/** history changeset selected */
	selectChangeset: ChangesetSelection;

	/** history changeset hovered over */
	hoverChangeset: ChangesetSelection;
};

export type DocumentCommands = {
	// TODO
	// scroll to heading
	// apply formatting
	// dispatch editor transaction
	// scrollToHeading: { text: string };
	// applyFormat: { wrap: string };
	// openLinkModal: void;
	// exportMarkdown: void;
	// exportHtml: void;
};

export type DocumentState = {
	// channel: ChannelT;
	// branchId: string;
	// selectedSeq: ChangesetSelection | null;
	// hoverSeq: ChangesetSelection | null;

	// /** currently focused branch */
	// branchId: string;

	// /** per-branch data */
	// branches: Record<string, DocumentBranchState>;

	controller: DocumentController;
	events: GlobalEmitter<DocumentEvents>;
	commands: GlobalEmitter<DocumentCommands>;
};

// export type DocumentBranchState = {
// 	doc: Y.Doc;
// 	editorState: EditorState;
// 	scrollTop: number;
// };

export const DocumentContext = createContext<DocumentState>();

export type DocumentProviderProps = ParentProps & {
	// channel: ChannelT;
};

export const DocumentProvider = (props: DocumentProviderProps) => {
	const controller = createDocumentController();

	const state: DocumentState = {
		controller,
		commands: controller.commands,
		events: controller.events,
	};

	return (
		<DocumentContext.Provider value={state}>
			{props.children}
		</DocumentContext.Provider>
	);
};

export const useDocument = (): DocumentState => {
	const ctx = useContext(DocumentContext);
	if (!ctx) {
		throw new Error(
			"useDocument must be used within a DocumentContext.Provider",
		);
	}
	return ctx;
};

export const createDocumentController = (): DocumentController => {
	const events = createGlobalEmitter<DocumentEvents>();
	const commands = createGlobalEmitter<DocumentCommands>();

	return {
		// jumpToBottom(smooth = false) {
		// 	commands.emit("jumpToBottom", { smooth });
		// },
		// jumpToTop(smooth = false) {
		// 	commands.emit("jumpToTop", { smooth });
		// },
		// jumpToMessage(message_id, smooth = false, highlight = false) {
		// 	commands.emit("jumpToMessage", { message_id, smooth, highlight });
		// },
		// scrollBy(px: number, smooth = false) {
		// 	commands.emit("scrollBy", { px, smooth });
		// },
		// ackMessage(message_id: string) {
		// 	commands.emit("ackMessage", { message_id });
		// },

		events,
		commands,
	};
};

// TODO: use a queue?
// const queue = new Queue(async (task: TimelineTask) => {
// 		log.debug("execute task", task);
// });
