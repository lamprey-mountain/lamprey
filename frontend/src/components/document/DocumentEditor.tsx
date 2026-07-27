import type { Parser as MarkdownParser } from "@lamprey/markdown";
import {
	chainCommands,
	deleteSelection,
	joinBackward,
	joinForward,
	selectNodeBackward,
	selectNodeForward,
} from "prosemirror-commands";
import { keymap } from "prosemirror-keymap";
import { DOMParser } from "prosemirror-model";
import { EditorState } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { createEffect, createSignal, onCleanup } from "solid-js";
import {
	initProseMirrorDoc,
	redo,
	undo,
	ySyncPlugin,
	yUndoPlugin,
} from "y-prosemirror";
import * as Y from "yjs";
import { useApi } from "@/api";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { parser as markdownParser } from "@/lib/markdown";
import { createAutocompletePlugin } from "../features/editor/autocomplete-plugin";
import {
	createPastePlugin,
	createSubmitPlugin,
	submitPluginKey,
} from "../features/editor/core-plugins";
import { createDiffPlugin } from "../features/editor/diff-plugin";
import { cursorPlugin } from "../features/editor/editor-cursors";
import {
	base64UrlDecode,
	base64UrlEncode,
	createListContinueCommand,
	createWrapCommand,
} from "../features/editor/editor-utils";
import { createEmojiPlugin } from "../features/editor/emoji-plugin";
import {
	createMarkdownInputRulesPlugin,
	joinBlockquoteBackward,
	joinBlockquoteForward,
} from "../features/editor/input-rules-plugin";
import { createMarkdownHighlightPlugin } from "../features/editor/markdown-highlight-plugin";
import {
	createPlaceholderPlugin,
	placeholderPluginKey,
} from "../features/editor/mod";
import { createEditorNodeViews } from "../features/editor/node-views";
import { schema } from "../features/editor/schema";
import { createToolbarPlugin } from "../features/editor/toolbar-plugin";
import { useDocument } from "./context";

const domParser = DOMParser.fromSchema(schema);

let md: MarkdownParser;
markdownParser.then((p) => (md = p));

export type DocumentEditorProps = {
	channelId: string;
	branchId: string;

	disabled?: boolean;
	placeholder?: string;
};

export const DocumentEditor = (props: DocumentEditorProps) => {
	const api = useApi();
	const doc = useDocument();

	const ydoc = new Y.Doc();
	ydoc.on("update", (update, origin) => {
		if (origin && origin.key === "server") return;

		api.client.send({
			type: "DocumentEdit",
			channel_id: props.channelId,
			branch_id: props.branchId,
			update: base64UrlEncode(update),
		});
	});

	// HACK: unsubscribe from ALL documents, then resubscribe to force the server to resend Document data
	// if i try to send DocumentSubscribe but im already subscribed the server wont do anything
	// PERF: reuse ydocs instead of resubscribing from scratch every time
	api.client.send({
		type: "Subscribe",
		documents: [],
	});
	api.client.send({
		type: "Subscribe",
		documents: [
			{
				channel_id: props.channelId,
				branch_id: props.branchId,
				state_vector: base64UrlEncode(Y.encodeStateVector(ydoc)),
			},
		],
	});

	api.events.on("sync", ([sync]) => {
		if (sync.type === "DocumentEdit") {
			if (sync.channel_id !== props.channelId) return;
			if (sync.branch_id !== props.branchId) return;
			const update = (
				(sync.update as unknown) instanceof Uint8Array
					? sync.update
					: base64UrlDecode(sync.update)
			) as Uint8Array;
			Y.applyUpdate(ydoc, update, { key: "server" });
		} else if (sync.type === "DocumentPresence") {
		} else if (sync.type === "DocumentSubscribed") {
			if (sync.channel_id !== props.channelId) return;
			if (sync.branch_id !== props.branchId) return;
			// setIsSubscribed(true);
		}
	});

	const type = ydoc.get("doc", Y.XmlFragment);
	const mapping = initProseMirrorDoc(type, schema).mapping;

	// TODO: use this?
	const [diffMarks, setDiffMarks] = createSignal([]);

	const toolbar = useFormattingToolbar();
	const toolbarPlugin = createToolbarPlugin(toolbar);

	const autocomplete = useAutocomplete();
	const autocompletePlugin = createAutocompletePlugin(
		autocomplete,
		() => props.channelId,
		() => "FIXME",
	);

	const emojiPlugin = createEmojiPlugin();

	const createState = () => {
		// NOTE: opts.initialContent isn't supported here
		// PERF: surely there's a better way than with DOMParser
		const doc = domParser.parse(document.createElement("div"));

		let selection;
		// if (doc && opts.initialSelection) {
		// 	let pos = 1;
		// 	if (opts.initialSelection === "end") {
		// 		pos = doc.content.size - 1;
		// 	}
		// 	selection = TextSelection.create(doc, pos);
		// }

		return EditorState.create({
			doc,
			selection,
			schema,
			plugins: [
				ySyncPlugin(type, { mapping }),
				cursorPlugin(
					api,
					props.channelId,
					props.branchId,
					// () => isSubscribed,
					// () => !(opts.diffMode?.() ?? false),
					() => true,
					() => true,
				),
				yUndoPlugin(),
				createPlaceholderPlugin(),
				createDiffPlugin(diffMarks),
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
					"Ctrl-m": (_state) => {
						return false;
					},
					Enter: (state, dispatch) => {
						return createListContinueCommand()(state, dispatch);
					},
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
					// ...opts.keymap,
				}),
				toolbarPlugin,
				autocompletePlugin,
				emojiPlugin,
			],
		});
	};

	const init = (el: HTMLDivElement) => {
		const state = createState();
		const view = new EditorView(el, {
			state,
			domParser,
			nodeViews: createEditorNodeViews()(),
			// handleDOMEvents,
			editable: () => !(props.disabled ?? false),
			dispatchTransaction(tr) {
				const newState = view.state.apply(tr);
				view.updateState(newState);

				// TODO: dispatch onChange?

				if (md) {
					// PERF: reuse parsed
					const parsed = md.parse(newState.doc.textContent);
					doc.setHeadings(parsed.headers());
				}
			},
		});

		doc.setEditor(view);
		onCleanup(() => {
			view.destroy();
			doc.setEditor(null);
		});

		// disable submit on enter
		view.dispatch(
			view.state.tr.setMeta(submitPluginKey, {
				submitOnEnter: false,
			}),
		);

		// reactively update placeholder
		createEffect(() => {
			const tr = view.state.tr.setMeta(placeholderPluginKey, props.placeholder);
			view.dispatch(tr);
		});

		// TODO: make these document commands?
		// view.dispatch();
		// view.update();
		// view.updateState();
	};

	return (
		<div
			class="editor"
			classList={{ disabled: props.disabled ?? false }}
			ref={init}
			role="textbox"
			aria-label="document editor"
			aria-placeholder={props.placeholder}
			aria-multiline="true"
		></div>
	);
};
