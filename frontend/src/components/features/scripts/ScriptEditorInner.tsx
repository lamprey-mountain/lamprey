import {
	// autocompletion, completionKeymap, closeBrackets,
	closeBracketsKeymap,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { HighlightStyle } from "@codemirror/highlight";
// import {
//   EditorView, keymap, highlightSpecialChars, drawSelection,
//   highlightActiveLine, dropCursor, rectangularSelection,
//   crosshairCursor, lineNumbers, highlightActiveLineGutter
// } from "@codemirror/view"
import {
	syntaxHighlighting,
	// defaultHighlightStyle, syntaxHighlighting, indentOnInput,
	// bracketMatching, foldGutter, foldKeymap
} from "@codemirror/language";
import { EditorState, Extension } from "@codemirror/state";
import {
	drawSelection,
	EditorView,
	keymap,
	lineNumbers,
} from "@codemirror/view";
import { onMount } from "solid-js";
import { syntaxHighlightingPlugin } from "../search";

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
	// TODO: copy frontend/src/styles/code.scss
	// {tag: tags.keyword, color: "#fc6"},
	// {tag: tags.comment, color: "#f5d", fontStyle: "italic"}
]);

export const CodeEditor = () => {
	let editorRef!: HTMLDivElement;

	onMount(() => {
		const view = new EditorView({
			doc: "Start document",
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
				// syntaxHighlighting(defaultHighlightStyle),
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
				// syntaxHighlighting(highlight),
			],
		});

		console.log(view);
	});

	return <div ref={editorRef!}></div>;
};
