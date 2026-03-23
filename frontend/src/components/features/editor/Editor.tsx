import { type Command, EditorState, TextSelection } from "prosemirror-state";
import { DOMParser } from "prosemirror-model";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { createEditor as createBaseEditor } from "./mod.tsx";
import { schema } from "./schema.ts";
import { md } from "../../../markdown_utils.tsx";
import {
	createListContinueCommand,
	createWrapCommand,
} from "./editor-utils.ts";
import { createToolbarPlugin } from "./toolbar-plugin.ts";
import { createAutocompletePlugin } from "./autocomplete-plugin.ts";
import { createEmojiPlugin } from "./emoji-plugin.ts";
import { useApi2, useChannels2 } from "@/api";
import { createEditorNodeViews } from "./node-views.tsx";
import {
	createMarkdownInputRulesPlugin,
	joinBlockquoteBackward,
	joinBlockquoteForward,
} from "./input-rules-plugin.ts";
import {
	chainCommands,
	deleteSelection,
	joinBackward,
	joinForward,
	selectNodeBackward,
	selectNodeForward,
} from "prosemirror-commands";
import { createPastePlugin, createSubmitPlugin } from "./core-plugins.ts";

let isApplyingFormat = false;
export const setIsApplyingFormat = (value: boolean) => {
	isApplyingFormat = value;
};
export const getIsApplyingFormat = () => isApplyingFormat;

export { schema };

type EditorProps = {
	initialContent?: string;
	keymap?: { [key: string]: Command };
	initialSelection?: "start" | "end";
	mentionRenderer?: (node: HTMLElement, userId: string) => void;
	mentionChannelRenderer?: (node: HTMLElement, channelId: string) => void;
};

export const createEditor = (
	opts: EditorProps & { channelId: () => string; roomId?: () => string },
) => {
	const api2 = useApi2();
	const channels2 = useChannels2();
	const toolbarPlugin = createToolbarPlugin();
	const autocompletePlugin = createAutocompletePlugin(
		opts.channelId,
		opts.roomId ?? (() => ""),
	);
	const emojiPlugin = createEmojiPlugin();

	const createState = () => {
		let doc;
		if (opts.initialContent) {
			const div = document.createElement("div");
			div.innerHTML = md.parser(md.lexer(opts.initialContent));
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
					"Backspace": chainCommands(
						deleteSelection,
						joinBlockquoteBackward,
						joinBackward,
						selectNodeBackward,
					),
					"Delete": chainCommands(
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
		nodeViews: createEditorNodeViews(api2, channels2, {
			currentChannelId: opts.channelId,
		}),
	});

	return {
		...editor,
		View: (props: Parameters<typeof editor.View>[0]) => {
			return <editor.View {...props} />;
		},
	};
};
