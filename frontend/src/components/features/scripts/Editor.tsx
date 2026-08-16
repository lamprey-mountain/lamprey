import {
	autocompletion,
	CompletionContext,
	// autocompletion, completionKeymap, closeBrackets,
	closeBracketsKeymap,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { javascript } from "@codemirror/lang-javascript";
import {
	HighlightStyle,
	syntaxHighlighting,
	// indentOnInput,
	// bracketMatching, foldGutter, foldKeymap
} from "@codemirror/language";
import { Compartment, EditorState, Extension } from "@codemirror/state";
import {
	Decoration,
	DecorationSet,
	drawSelection,
	EditorView,
	highlightActiveLine,
	keymap,
	lineNumbers,
	MatchDecorator,
	placeholder,
	ViewPlugin,
	ViewUpdate,
	WidgetType,
} from "@codemirror/view";
import { createEffect, createResource, createSignal, onMount } from "solid-js";
import type { Script } from "ts-sdk";
import { yCollab } from "y-codemirror.next";
import * as Y from "yjs";
import { useApi } from "@/api";
import { getGetUrl } from "@/media/util";
import { useScript } from "./context";
import { highlight, theme } from "./theme";

// import {
//   searchKeymap, highlightSelectionMatches
// } from "@codemirror/search";
// import {lintKeymap} from "@codemirror/lint"

export const CodeEditor = (props: {
	script: Script;
	onChange?: (val: string) => void;
}) => {
	const api = useApi();
	const scriptContext = useScript();

	const getUrl = getGetUrl();

	const [loading, setLoading] = createSignal(true);

	let editorRef!: HTMLDivElement;
	let view: EditorView;
	const stateConfigCompartment = new Compartment();

	const [mediaContent] = createResource(
		() => props.script,
		async (s) => {
			if (s.latest_version.location.type === "Hosted") {
				return fetch(getUrl(s.latest_version.location.media)).then((r) =>
					r.text(),
				);
			}
			return undefined;
		},
	);

	createEffect(() => {
		if (props.script.latest_version.location.type === "Document") {
			setLoading(scriptContext.isSubscribed(props.script.id));
		} else {
			setLoading(mediaContent.loading);
		}
	});

	onMount(() => {
		const extensions = [
			lineNumbers(),
			// foldGutter(),
			// highlightSpecialChars(),
			drawSelection(),
			// dropCursor(),
			EditorState.allowMultipleSelections.of(true),
			history(),
			// // Show a drop cursor when dragging over the editor
			// // Allow multiple cursors/selections
			// // Re-indent lines when typing specific input
			// indentOnInput(),
			// // Highlight syntax with a default style
			// // Highlight matching brackets near cursor
			// bracketMatching(),
			// // Automatically close brackets
			// closeBrackets(),
			// // Load the autocompletion system
			// autocompletion(),
			// // Allow alt-drag to select rectangular regions
			// rectangularSelection(),
			// // Change the cursor to a crosshair when holding alt
			// crosshairCursor(),
			highlightActiveLine(),
			// // Style the gutter for current line specially
			// highlightActiveLineGutter(),
			// // Highlight text that matches the selected text
			// highlightSelectionMatches(),
			keymap.of([
				...closeBracketsKeymap,
				...defaultKeymap,
				// ...searchKeymap,
				...historyKeymap,
				// ...foldKeymap,
				// ...completionKeymap,
				// ...lintKeymap
			]),
			theme,
			javascript(),
			syntaxHighlighting(highlight),
			stateConfigCompartment.of([
				EditorView.editable.of(!loading()),
				EditorState.readOnly.of(loading()),
			]),
			EditorView.updateListener.of((update) => {
				if (update.docChanged && props.onChange) {
					props.onChange(update.state.doc.toString());
				}
			}),
		];

		const ydoc = scriptContext.acquire(props.script);
		if (ydoc) {
			// const ytype = ydoc.get("doc", Y.XmlFragment);
			const ytype = ydoc.getText("doc");
			const undoManager = new Y.UndoManager(ytype);
			extensions.push(yCollab(ytype, null, { undoManager }));
		} else {
			// TODO(?): move mediaContent-specific logic here
		}

		view = new EditorView({
			parent: editorRef,
			extensions,
		});
	});

	// sync source text for media sources
	createEffect(() => {
		if (!view) return;

		const ydoc = scriptContext.acquire(props.script);
		if (ydoc) return;

		// TODO: show indicator when media changes, button to reload mediaContent
		const nextDoc = mediaContent();
		const currentDoc = view.state.doc.toString();

		if (nextDoc !== undefined && currentDoc !== nextDoc) {
			view.dispatch({
				changes: { from: 0, to: currentDoc.length, insert: nextDoc },
			});
		}
	});

	// disable editor when loading
	createEffect(() => {
		view.dispatch({
			effects: stateConfigCompartment.reconfigure([
				EditorView.editable.of(!loading()),
				EditorState.readOnly.of(loading()),
			]),
		});
	});

	return <div ref={editorRef!}></div>;
};
