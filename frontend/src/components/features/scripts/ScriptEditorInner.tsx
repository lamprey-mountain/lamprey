import {
	autocompletion,
	CompletionContext,
	// autocompletion, completionKeymap, closeBrackets,
	closeBracketsKeymap,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import {
	HighlightStyle,
	syntaxHighlighting,
	// indentOnInput,
	// bracketMatching, foldGutter, foldKeymap
} from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import { EditorState, Extension, Compartment } from "@codemirror/state";
import {
	Decoration,
	DecorationSet,
	drawSelection,
	EditorView,
	keymap,
	lineNumbers,
	MatchDecorator,
	placeholder,
	ViewPlugin,
	ViewUpdate,
	WidgetType,
} from "@codemirror/view";
import { onMount, createEffect } from "solid-js";
import { syntaxHighlightingPlugin } from "../search";
import { javascript } from "@codemirror/lang-javascript";

// import {
//   searchKeymap, highlightSelectionMatches
// } from "@codemirror/search"
// import {lintKeymap} from "@codemirror/lint"

const theme = EditorView.theme(
	{
		"&": {
			color: "oklch(var(--color-fg2))",
		},
		".cm-content": {
			fontFamily: "inherit",
			caretColor: "#ff0000",
		},
		"&.cm-focused .cm-selectionBackground, ::selection": {
			// backgroundColor: "#3fa9c9",
			backgroundColor: "#f00",
		},
		".cm-gutters": {
			backgroundColor: "oklch(var(--color-bg1))",
			color: "oklch(var(--color-fg2))",
			// border: "none"
		},
	},
	{ dark: true },
);

const highlight = HighlightStyle.define([
	{ tag: [t.comment, t.quote], color: "oklch(var(--color-fg6))", fontStyle: "italic" },
	{ tag: [t.keyword, t.modifier, t.inserted], color: "oklch(var(--color-magenta))" },
	{ tag: [t.number, t.string, t.bool, t.regexp, t.literal], color: "oklch(var(--color-green))" },
	{ tag: [t.heading, t.name, t.className, t.tagName], color: "oklch(var(--color-blue))" },
	{ tag: [t.attributeName, t.propertyName, t.variableName, t.typeName], color: "oklch(var(--color-yellow))" },
	{ tag: [t.atom, t.meta, t.link], color: "oklch(var(--color-orange))" },
	{ tag: [t.deleted, t.standard(t.name)], color: "oklch(var(--color-red))" },
	{ tag: t.emphasis, fontStyle: "italic" },
	{ tag: t.strong, fontWeight: "bold" },
]);

export const CodeEditor = (props: { source?: string; loading?: boolean }) => {
	let editorRef!: HTMLDivElement;
	let view: EditorView;
	const stateConfigCompartment = new Compartment();

	onMount(() => {
		view = new EditorView({
			doc: props.loading ? "Loading..." : (props.source ?? ""),
			parent: editorRef,
			extensions: [
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
				// // Style the current line specially
				// highlightActiveLine(),
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
					EditorView.editable.of(!props.loading),
					EditorState.readOnly.of(props.loading ?? false)
				]),
			],
		});
	});

	createEffect(() => {
		if (!view) return;
		const loading = props.loading ?? false;
		const nextDoc = loading ? "Loading..." : (props.source ?? "");
		const currentDoc = view.state.doc.toString();
		
		if (currentDoc !== nextDoc) {
			view.dispatch({
				changes: { from: 0, to: currentDoc.length, insert: nextDoc }
			});
		}
		
		view.dispatch({
			effects: stateConfigCompartment.reconfigure([
				EditorView.editable.of(!loading),
				EditorState.readOnly.of(loading)
			])
		});
	});

	return <div ref={editorRef!}></div>;
};
