import { type Command, EditorState, TextSelection } from "prosemirror-state";
import { DOMParser } from "prosemirror-model";
import { EditorView } from "prosemirror-view";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import { createEditor as createBaseEditor } from "./mod.tsx";
import { schema } from "./schema.ts";
import { md } from "../markdown_utils.tsx";
import {
	createListContinueCommand,
	createWrapCommand,
} from "./editor-utils.ts";
import { useFormattingToolbar } from "../contexts/formatting-toolbar.tsx";
import { setFormattingToolbarView } from "../contexts/FormattingToolbar.tsx";
import { createToolbarPlugin } from "./toolbar-plugin.ts";

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

export const createEditor = (opts: EditorProps) => {
	const toolbarPlugin = createToolbarPlugin();

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
				keymap({
					"Ctrl-z": undo,
					"Ctrl-Shift-z": redo,
					"Ctrl-y": redo,
					"Ctrl-b": createWrapCommand("**"),
					"Ctrl-i": createWrapCommand("*"),
					"Ctrl-`": createWrapCommand("`"),
					"Ctrl-m": (_state) => {
						return false;
					},
					"Shift-Enter": createListContinueCommand(),
					"Enter": (state, dispatch) => {
						// This is handled by mod.tsx but we keep list continue for Shift-Enter
						return false;
					},
					"Backspace": (state, dispatch) => {
						const sel = state.tr.selection;
						if (sel.empty) {
							const pos = sel.$anchor.pos - 1;
							if (pos >= 0) {
								dispatch?.(state.tr.deleteRange(pos, pos + 1));
							}
						} else {
							dispatch?.(state.tr.deleteSelection());
						}
						return true;
					},
					...opts.keymap,
				}),
				toolbarPlugin,
			],
		});
	};

	const editor = createBaseEditor({
		schema,
		createState: () => createState(),
		nodeViews: () => ({
			mention: (node: any) => {
				const dom = document.createElement("span");
				dom.classList.add("mention");
				if (opts.mentionRenderer) {
					opts.mentionRenderer(dom, node.attrs.user);
				} else {
					dom.textContent = `@${node.attrs.user}`;
				}
				return { dom };
			},
			mentionChannel: (node: any) => {
				const dom = document.createElement("span");
				dom.classList.add("mention");
				if (opts.mentionChannelRenderer) {
					opts.mentionChannelRenderer(dom, node.attrs.channel);
				} else {
					dom.textContent = `#${node.attrs.channel}`;
				}
				return { dom };
			},
		}),
	});

	return {
		...editor,
		View: (props: Parameters<typeof editor.View>[0]) => {
			return <editor.View {...props} />;
		},
	};
};
