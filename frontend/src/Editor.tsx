import { Command, EditorState, TextSelection } from "prosemirror-state";
import { Decoration, DecorationSet, EditorView } from "prosemirror-view";
import { DOMParser, Schema, Slice } from "prosemirror-model";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { marked, Token } from "marked";
import { createEffect, onCleanup, onMount } from "solid-js";

const md = marked.use({ breaks: true });

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
		text: {
			group: "inline",
			inline: true,
		},
	},
});

type EditorProps = {
	onSubmit: (_: { text: string; html: string }) => void;
	placeholder?: string;
	disabled?: boolean;
};

export const Editor = (props: EditorProps) => {
	let editorEl: HTMLDivElement;

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

	onMount(() => {
		const view = new EditorView({ mount: editorEl }, {
			domParser: DOMParser.fromSchema(schema),
			state: EditorState.create({
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
						"Ctrl-m": (state) => {
							console.log(state.doc);
							return false;
						},
						"Shift-Enter": (state, dispatch) => {
							dispatch?.(state.tr.insertText("\n"));
							return true;
						},
						"Enter": (state, dispatch) => {
							const html = md(state.doc.textContent.trim()).trim();
							// console.log({
							//   text: state.doc.textContent,
							//   html,
							// });
							// FIXME: marked adds extra newlines
							// i might need to write my own parser
							const res = props.onSubmit({
								text: state.doc.textContent.trim(),
								html,
							});
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
			}),
			decorations(state) {
				if (state.doc.firstChild!.firstChild === null) {
					const placeholder = (
						<div class="absolute text-fg4">{/* @once */ props.placeholder}</div>
					) as HTMLDivElement;
					return DecorationSet.create(state.doc, [
						Decoration.widget(0, placeholder),
					]);
				}

				const decorations: Array<Decoration> = [];
				let pos = 0;

				// TODO(refactor): using marked for decorations is pretty brittle
				function walk(ast: Token) {
					switch (ast.type) {
						case "paragraph":
							ast.tokens?.forEach(walk);
							return;
						case "text": {
							if ("tokens" in ast) {
								ast.tokens?.forEach(walk);
								return;
							} else {
								break;
							}
						}
						case "heading": {
							decorations.push(
								Decoration.inline(pos + 1, pos + ast.depth + 1, {
									class: "syn",
								}),
							);
							decorations.push(
								Decoration.inline(
									pos + ast.depth + 1,
									pos + ast.raw.length + 1,
									{ class: "header" },
								),
							);
							break;
						}
						case "em": {
							const end = pos + ast.raw.length + 1;
							decorations.push(
								Decoration.inline(pos + 1, pos + 2, { class: "syn" }),
							);
							decorations.push(
								Decoration.inline(pos + 2, end - 1, { nodeName: "em" }),
							);
							decorations.push(
								Decoration.inline(end - 1, end, { class: "syn" }),
							);
							pos += 1;
							ast.tokens?.forEach(walk);
							pos += 1;
							return;
						}
						case "strong": {
							const end = pos + ast.raw.length + 1;
							decorations.push(
								Decoration.inline(pos + 1, pos + 3, { class: "syn" }),
							);
							decorations.push(
								Decoration.inline(pos + 3, end - 2, { nodeName: "strong" }),
							);
							decorations.push(
								Decoration.inline(end - 2, end, { class: "syn" }),
							);
							break;
						}
						case "link": {
							if (ast.raw === ast.href) {
								const hrefLen = ast.href.length;
								decorations.push(
									Decoration.inline(pos + 1, pos + hrefLen + 1, {
										class: "link",
									}),
								);
							} else {
								const textLen = ast.text.length;
								const hrefLen = ast.href.length;
								decorations.push(
									Decoration.inline(pos + 1, pos + 2, { class: "syn" }),
								);
								decorations.push(
									Decoration.inline(pos + textLen + 2, pos + textLen + 4, {
										class: "syn",
									}),
								);
								decorations.push(
									Decoration.inline(
										pos + textLen + 4,
										pos + textLen + hrefLen + 4,
										{ class: "link" },
									),
								);
								decorations.push(
									Decoration.inline(
										pos + textLen + hrefLen + 4,
										pos + textLen + hrefLen + 5,
										{ class: "syn" },
									),
								);
							}
							break;
						}
						// // @ts-ignore
						// case "inlineKatex": {
						//   const macroRegex = /\\\w+/gi;
						//   const braceRegex = /\{\}/gi;
						//   let match;
						//   while (match = braceRegex.exec(ast.text)) {
						//     decorations.push(Decoration.inline(pos + match.index + 2, pos + match.index + match[0].length + 2, { class: "syn" }));
						//   }
						//   while (match = macroRegex.exec(ast.text)) {
						//     decorations.push(Decoration.inline(pos + match.index + 2, pos + match.index + match[0].length + 2, { class: "bold" }));
						//   }
						// }
						case "codespan": {
							const end = pos + ast.raw.length + 1;
							decorations.push(
								Decoration.inline(pos + 1, pos + 2, { class: "syn" }),
							);
							decorations.push(
								Decoration.inline(pos + 2, end - 1, { nodeName: "code" }),
							);
							decorations.push(
								Decoration.inline(end - 1, end, { class: "syn" }),
							);
							break;
						}
						case "code": {
							const end = pos + ast.raw.length + 1;
							// FIXME: indented code blocks...
							const syn = ast.raw.match(/(^`+)(.*)/);
							if (!syn) break;
							const synEndLen = ast.raw.match(/(`+\s*$)/)?.[0].length ?? 0;
							const synLen = syn[1].length;
							const codeLen = syn[2].length;
							// syntax highlighting?
							decorations.push(
								Decoration.inline(pos + 1, pos + synLen + 1, { class: "syn" }),
							);
							decorations.push(
								Decoration.inline(pos + synLen + codeLen + 2, end - synEndLen, {
									nodeName: "pre",
								}),
							);
							decorations.push(
								Decoration.inline(end - synEndLen, end, { class: "syn" }),
							);
							break;
						}
						case "blockquote": {
							// // FIXME: breaks on multiline blockquotes "> foo\n> bar"
							// const synLen = ast.raw.length - ast.text.length;
							// decorations.push(Decoration.inline(pos, pos + synLen, { class: "syn" }));
							// pos += synLen;
							// ast.tokens?.forEach(walk);

							// FIXME: format recursively using ast.tokens trickery or a better library
							// console.log({ ast })
							for (const line of ast.raw.split("\n")) {
								// console.log({ pos, line })
								if (line.startsWith(">")) {
									decorations.push(
										Decoration.inline(pos + 1, pos + 2, { class: "syn" }),
									);
								}
								pos += line.length + 1;
								// ast.tokens?.forEach(walk);
							}
							return;
						}
						case "list": {
							ast.items.forEach(walk);
							return;
						}
						case "list_item": {
							const endLen = ast.raw.match(/\n+$/)?.[0].length ?? 0;
							const startLen = ast.raw.length - ast.text.length - endLen;
							decorations.push(
								Decoration.inline(pos, pos + startLen, { class: "syn" }),
							);
							pos += startLen;
							ast.tokens?.forEach(walk);
							pos += endLen;
							return;
						}
					}
					pos += ast.raw.length;
				}

				// console.log(md.lexer(state.doc.textContent));
				md.lexer(state.doc.textContent).forEach(walk);
				return DecorationSet.create(state.doc, decorations);
			},
			handlePaste(view, event, slice) {
				// const files = event.clipboardData?.files ?? [];
				// for (const file of files) props.onUpload?.(file);
				const str = slice.content.textBetween(0, slice.size);
				const tr = view.state.tr;
				if (/^(https?:\/\/|mailto:)\S+$/i.test(str) && !tr.selection.empty) {
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
						tr.replaceSelection(slice).scrollIntoView().setMeta("paste", true)
							.setMeta("uiEvent", "paste"),
					);
					// view.dispatch(tr.deleteSelection().insertText(str).scrollIntoView().setMeta("paste", true).setMeta("uiEvent", "paste"));
				}
				return true;
			},
			transformPastedHTML(html) {
				const tmp = document.createElement("x-html-import");
				tmp.innerHTML = html;

				const escape = (s: string) => s;
				const replacements: Array<[string, (el: HTMLElement) => string]> = [
					[
						"b, bold, strong, [style*='font-weight:700']",
						(el) => `**${el.textContent}**`,
					],
					[
						"em, i, [style*='font-style:italic']",
						(el) => `*${el.textContent}*`,
					],
					["a", (el) => `[${el.textContent}](${el.getAttribute("href")})`],
					["code", (el) => `\`${el.textContent}\``],
					...[1, 2, 3, 4, 5, 6].map((
						level,
					) => [
						"h" + level,
						(el: HTMLElement) => `${"#".repeat(level)} ${el.textContent}`,
					]) as Array<any>,
					[
						"ul",
						(el) =>
							"\n" + [...el.children].map((i) =>
								"- " + i.textContent
							).join("\n") + "\n",
					],
					[
						"ol",
						(el) =>
							"\n" + [...el.children].map((i, x) =>
								(x + 1) + ". " + i.textContent
							).join("\n") + "\n",
					],
					["blockquote", (el) => "\n> " + el.textContent + "\n\n"],
					["br", () => "\n"],
				];

				// function walk(node, preserveSpace = false) {
				function walk(node, preserveSpace = true) {
					// HACK: i don't know if some text should be pre formatted or not, so i assume not unless its in a pre
					// but this might mangle text thats copied from within the app, since everything is preformatted
					// DISABLED FOR NOW
					node.childNodes.forEach((el) =>
						walk(el, node.nodeName === "PRE" ? true : preserveSpace)
					);
					if (node.nodeType === Node.TEXT_NODE) {
						if (preserveSpace) {
							node.replaceWith(escape(node.textContent));
						} else {
							node.replaceWith(escape(node.textContent.replace(/\s+/g, " ")));
						}
						return;
					}
					for (const [match, fn] of replacements) {
						if (node.matches(match)) {
							node.replaceWith(fn(node));
							break;
						}
					}
				}

				walk(tmp);
				console.log({ from: html, to: tmp.outerHTML });
				return tmp.outerHTML;
			},
		});
		view.focus();
		createEffect(() => {
			view.setProps({
				// HACK: make prosemirror update properly
				editable: () => !(props.disabled ?? false),
			});
			// if (props.disabled)
		});
		onCleanup(() => view.destroy());
	});

	return (
		<div
			class="bg-bg3 flex-1 border-[1px] border-sep px-[4px] whitespace-pre-wrap overflow-y-auto"
			classList={{ "bg-bg4": props.disabled ?? false }}
			tabindex={0}
			ref={editorEl!}
		>
		</div>
	);
};

export default Editor;
