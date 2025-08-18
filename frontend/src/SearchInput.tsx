import {
	type Command,
	EditorState,
	Plugin,
	TextSelection,
} from "prosemirror-state";
import {
	Decoration,
	type DecorationAttrs,
	DecorationSet,
	EditorView,
} from "prosemirror-view";
import { DOMParser, Schema } from "prosemirror-model";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { onCleanup, onMount } from "solid-js";

async function fetchAuthors(query: string) {
	return Promise.resolve([
		{ name: "alice", id: "1" },
		{ name: "bob", id: "2" },
	].filter((a) => a.name.includes(query)));
}

async function fetchLocations(query: string) {
	return Promise.resolve([
		{ name: "general", id: "10" },
		{ name: "random", id: "11" },
	].filter((l) => l.name.includes(query)));
}

const schema = new Schema({
	nodes: {
		doc: {
			content: "block+",
		},
		paragraph: {
			content: "inline*",
			group: "block",
			whitespace: "pre",
			toDOM: () => ["p", 0],
			parseDOM: [{
				tag: "p",
				preserveWhitespace: "full",
			}],
		},
		text: {
			group: "inline",
			inline: true,
		},
		atom: {
			group: "inline",
			inline: true,
			atom: true,
			attrs: { type: {}, value: {} },
			toDOM: (
				node,
			) => [
				"span",
				{ class: `atom atom-${node.attrs.type}` },
				`${node.attrs.type}:${node.attrs.value}`,
			],
			parseDOM: [{ tag: "span.atom", getAttrs: (dom) => ({}) }],
		},
	},
	marks: {
		quoted: {
			toDOM: () => ["span", { class: "quoted" }, 0],
			parseDOM: [{ tag: "span.quoted" }],
		},
		negated: {
			toDOM: () => ["span", { class: "negated" }, 0],
			parseDOM: [{ tag: "span.negated" }],
		},
	},
});

function highlightPlugin() {
	return new Plugin({
		props: {
			decorations(state) {
				const decorations: Decoration[] = [];
				const text = state.doc.textContent;

				// quoted
				const quoteRegex = /"([^"]*)"/g;
				let m;
				while ((m = quoteRegex.exec(text))) {
					decorations.push(
						Decoration.inline(m.index, m.index + m[0].length, {
							class: "quoted",
						}),
					);
				}

				// -not
				const notRegex = /-([^\s]+)/g;
				while ((m = notRegex.exec(text))) {
					decorations.push(
						Decoration.inline(m.index, m.index + m[0].length, {
							class: "negated",
						}),
					);
				}

				return DecorationSet.create(state.doc, decorations);
			},
		},
	});
}

function autocompletePlugin() {
	return new Plugin({
		props: {
			handleTextInput(view, from, to, text) {
				const before = view.state.doc.textBetween(0, from, " ");
				if (/\bhas:$/.test(before)) {
					// suggest fixed types
					console.log("Autocomplete options:", [
						"image",
						"audio",
						"video",
						"file",
					]);
				} else if (/\bauthor:$/.test(before)) {
					fetchAuthors("").then((r) => console.log("Author suggestions", r));
				} else if (/\bin:$/.test(before)) {
					fetchLocations("").then((r) =>
						console.log("Location suggestions", r)
					);
				}
				return false;
			},
		},
	});
}

export const SearchInput = () => {
	const state = EditorState.create({
		schema,
		plugins: [
			history(),
			keymap({
				"Ctrl-z": undo,
				"Ctrl-Shift-z": redo,
				"Ctrl-y": redo,
				"Enter": (state) => {
					console.log(state.doc.textContent.trim());
					return true;
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
			}),
			highlightPlugin(),
			autocompletePlugin(),
		],
	});

	let editorEl: HTMLDivElement | undefined;

	onMount(() => {
		const view = new EditorView({ mount: editorEl! }, {
			domParser: DOMParser.fromSchema(schema),
			state,
		});
		onCleanup(() => view.destroy());
	});

	return (
		<div
			class="editor"
			tabindex={0}
			ref={editorEl!}
			role="textbox"
			aria-label="search input"
			aria-placeholder="search..."
		>
		</div>
	);
};
