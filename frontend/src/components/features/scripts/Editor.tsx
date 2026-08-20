import {
	autocompletion,
	CompletionContext,
	closeBrackets,
	// autocompletion, completionKeymap, closeBrackets,
	closeBracketsKeymap,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { javascript } from "@codemirror/lang-javascript";
import {
	bracketMatching,
	foldGutter,
	HighlightStyle,
	indentOnInput,
	syntaxHighlighting,
	// indentOnInput,
	// bracketMatching, foldGutter, foldKeymap
} from "@codemirror/language";
import { highlightSelectionMatches } from "@codemirror/search";
import { Compartment, EditorState, Extension } from "@codemirror/state";
import {
	crosshairCursor,
	Decoration,
	DecorationSet,
	drawSelection,
	dropCursor,
	EditorView,
	highlightActiveLine,
	highlightActiveLineGutter,
	highlightSpecialChars,
	keymap,
	lineNumbers,
	MatchDecorator,
	placeholder,
	rectangularSelection,
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
import { cursorPlugin } from "./codemirror-editor-cursors";
import { useScript } from "./context";
import { highlight, theme } from "./theme";
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
			setLoading(!scriptContext.isSubscribed(props.script.id));
		} else {
			setLoading(mediaContent.loading);
		}
	});

	onMount(() => {
		const extensions = [
			drawSelection(),
			lineNumbers(),
			foldGutter(),
			highlightSpecialChars(),
			dropCursor(),
			EditorState.allowMultipleSelections.of(true),
			history(),
			indentOnInput(),
			bracketMatching(),
			closeBrackets(),
			autocompletion(),
			// FIXME: rectangular selection
			// rectangularSelection(),
			// crosshairCursor(),
			highlightActiveLine(),
			highlightSelectionMatches(),
			drawSelection(),
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
			javascript(), // TODO(future): swap this depending on language
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
			extensions.push(
				cursorPlugin(api, scriptContext.channel_id, props.script.id, ytype),
			);
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
