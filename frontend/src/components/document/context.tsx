import {
	createGlobalEmitter,
	type GlobalEmitter,
} from "@solid-primitives/event-bus";
import type { EditorView } from "prosemirror-view";
import {
	type Accessor,
	createContext,
	createSignal,
	type ParentProps,
	type Setter,
	useContext,
} from "solid-js";
import type { ChangesetSelection, HeaderItem } from "./types";

export type DocumentController = {
	events: GlobalEmitter<DocumentEvents>;
	commands: GlobalEmitter<DocumentCommands>;
};

export type DocumentEvents = {
	// ...?
};

export type DocumentCommands = {
	/** history changeset selected */
	selectChangeset: ChangesetSelection;

	/** history changeset hovered over */
	hoverChangeset: ChangesetSelection;

	// scroll to heading
	// apply formatting
	// dispatch editor transaction
	// scrollToHeading: { text: string };
	// applyFormat: { wrap: string };
	// openLinkModal: void;
	// exportMarkdown: void;
	// exportHtml: void;
};

// TODO: restore scroll position per document
// TODO: persist tocOpen in user preferences
// TODO: consider using a solidjs store for this state
export type DocumentState = {
	// mode: DocumentMode;

	branchId: Accessor<string>;
	setBranchId: Setter<string>;
	selectedSeq: Accessor<ChangesetSelection | null>;
	setSelectedSeq: Setter<ChangesetSelection | null>;
	hoverSeq: Accessor<ChangesetSelection | null>;
	setHoverSeq: Setter<ChangesetSelection | null>;
	headings: Accessor<HeaderItem[]>;
	setHeadings: Setter<HeaderItem[]>;
	tocOpen: Accessor<boolean>;
	setTocOpen: Setter<boolean>;
	editor: Accessor<EditorView | null>;
	setEditor: Setter<EditorView | null>;

	controller: DocumentController;
	events: GlobalEmitter<DocumentEvents>;
	commands: GlobalEmitter<DocumentCommands>;
};

export const DocumentContext = createContext<DocumentState>();

export type DocumentProviderProps = ParentProps & {
	initialBranchId: string;
};

export const DocumentProvider = (props: DocumentProviderProps) => {
	const controller = createDocumentController();
	const [headings, setHeadings] = createSignal<HeaderItem[]>([]);
	const [branchId, setBranchId] = createSignal(props.initialBranchId);
	const [selectedSeq, setSelectedSeq] = createSignal<ChangesetSelection | null>(
		null,
	);
	const [hoverSeq, setHoverSeq] = createSignal<ChangesetSelection | null>(null);
	const [tocOpen, setTocOpen] = createSignal(true);
	const [editor, setEditor] = createSignal<EditorView | null>(null);

	const state: DocumentState = {
		selectedSeq,
		setSelectedSeq,
		hoverSeq,
		setHoverSeq,
		branchId,
		setBranchId,
		controller,
		commands: controller.commands,
		events: controller.events,
		headings,
		setHeadings,
		tocOpen,
		setTocOpen,
		editor,
		setEditor,
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
