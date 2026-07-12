import {
	chainCommands,
	deleteSelection,
	joinBackward,
	joinForward,
	selectNodeBackward,
	selectNodeForward,
} from "prosemirror-commands";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { DOMParser } from "prosemirror-model";
import { type Command, EditorState, TextSelection } from "prosemirror-state";
import type { AutocompleteContext } from "@/contexts/autocomplete.tsx";
import type { FormattingToolbarContextT } from "@/contexts/formatting-toolbar.tsx";
import { md } from "@/lib/markdown";
import { createAutocompletePlugin } from "./autocomplete-plugin.ts";
import { createPastePlugin, createSubmitPlugin } from "./core-plugins.ts";
import {
	createListContinueCommand,
	createWrapCommand,
} from "./editor-utils.ts";
import { createEmojiPlugin } from "./emoji-plugin.ts";
import {
	createMarkdownInputRulesPlugin,
	joinBlockquoteBackward,
	joinBlockquoteForward,
} from "./input-rules-plugin.ts";
import { createMarkdownHighlightPlugin } from "./markdown-highlight-plugin.ts";
import {
	createEditor as createBaseEditor,
	createPlaceholderPlugin,
	type EditorViewProps,
} from "./mod.tsx";
import { createEditorNodeViews } from "./node-views.tsx";
import { schema } from "./schema.ts";
import { createToolbarPlugin } from "./toolbar-plugin.ts";

let isApplyingFormat = false;
export const setIsApplyingFormat = (value: boolean) => {
	isApplyingFormat = value;
};
export const getIsApplyingFormat = () => isApplyingFormat;

export { schema };

export type EditorProps = {
	initialContent?: () => string;
	keymap?: { [key: string]: Command };
	initialSelection?: "start" | "end";
	mentionRenderer?: (node: HTMLElement, userId: string) => void;
	mentionChannelRenderer?: (node: HTMLElement, channelId: string) => void;

	channelId: () => string;
	roomId?: () => string;
	toolbar?: FormattingToolbarContextT;
	autocomplete?: AutocompleteContext;
};

export type Editor = {
	schema: typeof schema;
	setState: (state?: EditorState) => void;
	state: () => EditorState | undefined;
	focus: () => void;
	view: any;
	View: (props: EditorViewProps) => any;
	createState: () => EditorState;
};

export const createEditor = (opts: EditorProps): Editor => {
	const toolbarPlugin = createToolbarPlugin(opts.toolbar!);
	const autocompletePlugin = createAutocompletePlugin(
		opts.autocomplete!,
		opts.channelId,
		opts.roomId ?? (() => ""),
	);
	const emojiPlugin = createEmojiPlugin();

	const createState = () => {
		let doc;
		if (opts.initialContent) {
			const div = document.createElement("div");
			div.innerHTML = md.parser(md.lexer(opts.initialContent()));
			doc = DOMParser.fromSchema(schema).parse(div);
		}

		let selection;
		if (doc && opts.initialSelection) {
			let pos = 1;
			if (opts.initialSelection === "end") {
				pos = doc.content.size - 1;
			}
			selection = TextSelection.create(doc, pos);
		}

		return EditorState.create({
			doc,
			selection,
			schema,
			plugins: [
				history(),
				createPlaceholderPlugin(),
				createMarkdownHighlightPlugin(),
				createMarkdownInputRulesPlugin(),
				createPastePlugin(),
				createSubmitPlugin(),
				keymap({
					"Ctrl-z": undo,
					"Ctrl-Shift-z": redo,
					"Ctrl-y": redo,
					"Ctrl-b": createWrapCommand("**"),
					"Ctrl-i": createWrapCommand("*"),
					"Ctrl-`": createWrapCommand("`"),
					"Shift-Enter": createListContinueCommand(),
					Backspace: chainCommands(
						deleteSelection,
						joinBlockquoteBackward,
						joinBackward,
						selectNodeBackward,
					),
					Delete: chainCommands(
						deleteSelection,
						joinBlockquoteForward,
						joinForward,
						selectNodeForward,
					),
					...opts.keymap,
				}),
				toolbarPlugin,
				autocompletePlugin,
				emojiPlugin,
			],
		});
	};

	const editor = createBaseEditor({
		schema,
		createState: () => createState(),
		nodeViews: createEditorNodeViews(),
	});

	return {
		...editor,
		createState,
		state: () => editor.view?.state,
		View: (props: Parameters<typeof editor.View>[0]) => {
			return <editor.View {...props} />;
		},
	};
};
