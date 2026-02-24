import { type Command, EditorState, TextSelection } from "prosemirror-state";
import { DOMParser } from "prosemirror-model";
import { EditorView } from "prosemirror-view";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import { createEditor as createBaseEditor } from "./mod.tsx";
import { schema } from "./schema.ts";
import { md } from "../markdown.tsx";
import {
	createListContinueCommand,
	createWrapCommand,
} from "./editor-utils.ts";
import { useFormattingToolbar } from "../contexts/formatting-toolbar.tsx";
import { setFormattingToolbarView } from "../contexts/FormattingToolbar.tsx";

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

const EditorWithToolbar = (props: { getView: () => EditorView }) => {
	const { showToolbar, hideToolbar } = useFormattingToolbar();
	let toolbarVisible = false;
	let initialized = false;
	let selectionRange: { from: number; to: number } | null = null;

	const updateToolbar = () => {
		const view = props.getView();
		if (!view) return;

		const { state } = view;
		const { empty, from, to } = state.selection;

		if (empty || from === to) {
			if (toolbarVisible && !isApplyingFormat) {
				hideToolbar();
				toolbarVisible = false;
				selectionRange = null;
			}
			return;
		}

		// Check if selection changed
		if (
			selectionRange?.from === from && selectionRange?.to === to &&
			toolbarVisible
		) {
			return;
		}
		selectionRange = { from, to };

		// Create a reference element for floating-ui
		const coords = view.coordsAtPos(from);
		const endCoords = view.coordsAtPos(to);

		const top = Math.min(coords.top, endCoords.top);
		const left = coords.left;
		const width = Math.max(1, endCoords.left - coords.left);
		const height = Math.max(
			coords.bottom - coords.top,
			endCoords.bottom - endCoords.top,
		);

		showToolbar({
			getBoundingClientRect() {
				return {
					x: left,
					y: top,
					width,
					height,
					left,
					top,
					right: left + width,
					bottom: top + height,
				};
			},
		});
		toolbarVisible = true;
	};

	onMount(() => {
		const view = props.getView();
		if (!view) return;

		setFormattingToolbarView(view);
		view.dom.addEventListener("selectionchange", updateToolbar);
		view.dom.addEventListener("keyup", updateToolbar);
		view.dom.addEventListener("mouseup", updateToolbar);
		initialized = true;

		onCleanup(() => {
			setFormattingToolbarView(null);
			view.dom.removeEventListener("selectionchange", updateToolbar);
			view.dom.removeEventListener("keyup", updateToolbar);
			view.dom.removeEventListener("mouseup", updateToolbar);
			if (toolbarVisible) hideToolbar();
		});
	});

	createEffect(() => {
		if (initialized) updateToolbar();
	});

	return null;
};

export const createEditor = (opts: EditorProps) => {
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
			],
		});
	};

	const editor = createBaseEditor({
		schema,
		createState: () => createState(),
		nodeViews: () => ({
			mention: (node) => {
				const dom = document.createElement("span");
				dom.classList.add("mention");
				if (opts.mentionRenderer) {
					opts.mentionRenderer(dom, node.attrs.user);
				} else {
					dom.textContent = `@${node.attrs.user}`;
				}
				return { dom };
			},
			mentionChannel: (node) => {
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
			return (
				<>
					<editor.View {...props} />
					<EditorWithToolbar getView={() => editor.view} />
				</>
			);
		},
	};
};
