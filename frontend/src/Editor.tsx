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
import { marked, type Token } from "marked";
import { createEffect, onCleanup, onMount } from "solid-js";
import { initTurndownService } from "./turndown.ts";
import { decorate } from "./markdown.tsx";

const turndown = initTurndownService();

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
			parseDOM: ["p", "x-html-import"].map((tag) => ({
				tag,
				preserveWhitespace: "full",
			})),
		},
		// maybe have special purpose blocks instead of pure markdown (markdown with incremental rich text)
		// blockquote: {},
		// table: {},
		// codeblock: {},
		// details: {},
		// media: {},
		mention: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				user: {},
			},
			toDOM: (
				n,
			) => ["span", { "data-mention": n.attrs.user }, `${n.attrs.user}`],
			parseDOM: [{
				tag: "span[data-mention]",
				getAttrs: (el) => ({ user: el.dataset.mention }),
			}],
		},
		emoji: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				id: {},
				shortname: {},
			},
			toDOM: (
				n,
			) => ["span", { "data-emoji-id": n.attrs.id }, `:${n.attrs.name}:`],
			parseDOM: [{
				tag: "span[data-emoji]",
				getAttrs: (el) => ({ id: el.dataset.id, shortname: el.dataset.emoji }),
			}],
		},
		text: {
			group: "inline",
			inline: true,
		},
	},
});

function createWrap(wrap: string): Command {
	const len = wrap.length;

	return (state, dispatch) => {
		const { from, to } = state.selection;
		const tr = state.tr;

		const isWrapped = (
			tr.doc.textBetween(from - len, from) === wrap &&
			tr.doc.textBetween(to, to + len) === wrap
		) || (
			false
			// FIXME: fails?
			// tr.doc.textBetween(from, from + len) === wrap &&
			// tr.doc.textBetween(to - len, to) === wrap
		);

		if (isWrapped) {
			tr.delete(to, to + len);
			tr.delete(from - len, from);
		} else {
			tr.insertText(wrap, to);
			tr.insertText(wrap, from);
			tr.setSelection(TextSelection.create(tr.doc, from + len, to + len));
		}

		dispatch?.(tr);
		return true;
	};
}

type EditorProps = {};

type EditorViewProps = {
	placeholder?: string;
	disabled?: boolean;
	onUpload?: (file: File) => void;
	onSubmit: (text: string) => void;
	onChange?: (state: EditorState) => void;
};

export const createEditor = (_opts: EditorProps) => {
	let editorRef!: HTMLDivElement;
	let view: EditorView | undefined;
	let onSubmit!: (content: string) => void | undefined;

	const createState = () =>
		EditorState.create({
			// doc: ..., // initial doc here?
			schema,
			plugins: [
				history(),
				keymap({
					"Ctrl-z": undo,
					"Ctrl-Shift-z": redo,
					"Ctrl-y": redo,
					"Ctrl-b": createWrap("**"),
					"Ctrl-i": createWrap("*"),
					"Ctrl-`": createWrap("`"),
					"Ctrl-m": (_state) => {
						return false;
					},
					"Shift-Enter": (state, dispatch) => {
						dispatch?.(state.tr.insertText("\n"));
						return true;
					},
					"Enter": (state, dispatch) => {
						// const html = (md(state.doc.textContent.trim()) as string).trim();
						// console.log({
						//   text: state.doc.textContent,
						//   html,
						// });
						// FIXME: marked adds extra newlines
						// i might need to write my own parser
						// const res = onSubmit({
						// 	text: state.doc.textContent.trim(),
						// 	html,
						// });
						onSubmit?.(state.doc.textContent.trim());
						// if (res !== false) dispatch?.(state.tr.deleteRange(0, state.doc.nodeSize - 2));
						// return !!res;
						// HACK: i don't know what this is, but i don't like it
						dispatch?.(state.tr.deleteRange(0, state.doc.nodeSize - 2));
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
			],
		});

	return {
		setState(state?: EditorState) {
			view?.updateState(state ?? createState());
		},
		focus() {
			view?.focus();
		},
		View(props: EditorViewProps) {
			createEffect(() => {
				onSubmit = props.onSubmit;
			});

			onMount(() => {
				view = new EditorView(editorRef!, {
					domParser: DOMParser.fromSchema(schema),
					state: createState(),
					decorations(state) {
						return decorate(state, props.placeholder);
					},
					handlePaste(view, event, slice) {
						const files = Array.from(event.clipboardData?.files ?? []);
						if (files.length) {
							for (const file of files) props.onUpload?.(file);
							return true;
						}
						const str = slice.content.textBetween(0, slice.size);
						const tr = view.state.tr;
						if (
							/^(https?:\/\/|mailto:)\S+$/i.test(str) && !tr.selection.empty
						) {
							tr.insertText("[", tr.selection.from);
							tr.insertText(`](${str})`, tr.selection.to);
							tr.setSelection(TextSelection.create(tr.doc, tr.selection.to));
							view.dispatch(
								tr.scrollIntoView().setMeta("paste", true).setMeta(
									"uiEvent",
									"paste",
								),
							);
						} else {
							// NOTE: is this correct? no, it isn't.
							// console.log(slice)
							// slice.content.
							view.dispatch(
								tr.replaceSelection(slice).scrollIntoView().setMeta(
									"paste",
									true,
								)
									.setMeta("uiEvent", "paste"),
							);
							// view.dispatch(tr.deleteSelection().insertText(str).scrollIntoView().setMeta("paste", true).setMeta("uiEvent", "paste"));
						}
						return true;
					},
					transformPastedHTML(html) {
						console.group("turndown");
						console.log("html", html);
						const md = turndown.turndown(html);
						console.log("markdown", md);
						console.groupEnd();

						const container = document.createElement("div");
						container.innerText = md;
						return container.outerHTML;
					},
					editable: () => !(props.disabled ?? false),
					dispatchTransaction(tr) {
						const newState = view!.state.apply(tr);
						view!.updateState(newState);
						props.onChange?.(newState);
					},
				});

				// console.log("editor mounted", editorRef, view);
				view.focus();
			});

			onCleanup(() => {
				view?.destroy();
			});

			createEffect(() => {
				// update when placeholder changes too
				props.placeholder;

				view?.setProps({
					editable: () => !(props.disabled ?? false),
				});
			});

			return (
				<div
					class="editor"
					classList={{ "disabled": props.disabled ?? false }}
					tabindex={0}
					ref={editorRef!}
					role="textbox"
					aria-label="chat input"
					aria-placeholder={props.placeholder}
					aria-multiline="true"
				>
				</div>
			);
		},
	};
};
