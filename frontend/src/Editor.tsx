import { Command, EditorState, TextSelection } from "prosemirror-state";
import { Decoration, DecorationAttrs, DecorationSet, EditorView } from "prosemirror-view";
import { DOMParser, Schema } from "prosemirror-model";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { marked, Token } from "marked";
import { createEffect, onCleanup, onMount } from "solid-js";
// @ts-types="npm:@types/sanitize-html@^2.13.0"
import sanitizeHtml from "npm:sanitize-html";

const md = marked.use({
	breaks: true,
	gfm: true,
});

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
	placeholder?: string;
	disabled?: boolean;
	class?: string;
	state: EditorState;
	onUpload?: (file: File) => void;
};

export function createEditorState(onSubmit: (text: string) => void) {
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
	
	return EditorState.create({
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
					onSubmit(state.doc.textContent.trim());
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
}

export const Editor = (props: EditorProps) => {
	let editorEl: HTMLDivElement;

	onMount(() => {
		const view = new EditorView({ mount: editorEl }, {
			domParser: DOMParser.fromSchema(schema),
			state: props.state,
			decorations(state) {
				if (state.doc.firstChild!.firstChild === null) {
					const placeholder = (
						<div class="placeholder" aria-hidden="true">{/* @once */ props.placeholder}</div>
					) as HTMLDivElement;
					return DecorationSet.create(state.doc, [
						Decoration.widget(0, placeholder),
					]);
				}

				function extraDecorations(ast: Token) {
					switch (ast.type) {
						case "heading": {
							return [{ attrs: { class: "syn" }, start: 0, end: ast.depth }];
						}
						case "em": {
							return [
								{ attrs: { class: "syn" }, start: 0, end: 1 },
								{ attrs: { nodeName: "em" }, start: 1, end: ast.raw.length - 1 },
								{ attrs: { class: "syn" }, start: ast.raw.length - 1, end: ast.raw.length },
							];
						}
						case "link": {
							if (ast.raw === ast.href) {
								return [{ attrs: { style: "color: var(--color-link)" }, start: 0, end: ast.text.length }]
							} else {
								return [
									{ attrs: { class: "syn" }, start: 0, end: 1 },
									{ attrs: { class: "syn" }, start: ast.text.length + 1, end: ast.text.length + 3 },
									{ attrs: { style: "color: var(--color-link)" }, start: ast.text.length + 3, end: ast.raw.length - 1 },
									{ attrs: { class: "syn" }, start: ast.raw.length - 1, end: ast.raw.length },
								]
							}
						}
						case "strong": {
							return [
								{ attrs: { class: "syn" }, start: 0, end: 2 },
								{ attrs: { nodeName: "b" }, start: 2, end: ast.raw.length - 2 },
								{ attrs: { class: "syn" }, start: ast.raw.length - 2, end: ast.raw.length },
							];
						}
						case "code": {
							// does this work with indented code blocks?
							const firstEnd = ast.raw.indexOf("\n");
							return [
								{ attrs: { class: "syn" }, start: 0, end: firstEnd },
								// { attrs: { nodeName: "pre" }, start: firstEnd + 1, end: ast.text.length + firstEnd + 1 },
								// { attrs: { class: "font-mono" }, start: firstEnd + 1, end: ast.text.length + firstEnd + 1 },
								{ attrs: { nodeName: "code" }, start: firstEnd + 1, end: ast.text.length + firstEnd + 1 },
								{ attrs: { class: "syn" }, start: ast.text.length + firstEnd + 2, end: ast.raw.length },
							];
						}
						case "codespan": {
							return [
								{ attrs: { class: "syn" }, start: 0, end: 1 },
								{ attrs: { nodeName: "code" }, start: 1, end: ast.raw.length - 1 },
								{ attrs: { class: "syn" }, start: ast.raw.length - 1, end: ast.raw.length },
							];
						}
						// case "blockquote": {
						// 	// // FIXME: breaks on multiline blockquotes "> foo\n> bar"
						// 	// const synLen = ast.raw.length - ast.text.length;
						// 	// decorations.push(Decoration.inline(pos, pos + synLen, { class: "syn" }));
						// 	// pos += synLen;
						// 	// ast.tokens?.forEach(walk);

						// 	// FIXME: format recursively using ast.tokens trickery or a better library
						// 	// console.log({ ast })
						// 	for (const line of ast.raw.split("\n")) {
						// 		// console.log({ pos, line })
						// 		if (line.startsWith(">")) {
						// 			decorations.push(
						// 				Decoration.inline(pos + 1, pos + 2, { class: "syn" }),
						// 			);
						// 		}
						// 		pos += line.length + 1;
						// 		// ast.tokens?.forEach(walk);
						// 	}
						// 	return;
						// }
						// case "list": {
						// 	ast.items.forEach(walk);
						// 	return;
						// }
						// case "list_item": {
						// 	const endLen = ast.raw.match(/\n+$/)?.[0].length ?? 0;
						// 	const startLen = ast.raw.length - ast.text.length - endLen;
						// 	decorations.push(
						// 		Decoration.inline(pos, pos + startLen, { class: "syn" }),
						// 	);
						// 	pos += startLen;
						// 	ast.tokens?.forEach(walk);
						// 	pos += endLen;
						// 	return;
						// }
						default: {
							return []
						}
					}
				}

				function getOffset(ty: string) {
					switch (ty) {
						case "strong": return 2;
						case "em": return 1;
						case "codespan": return 1;
						default: return 0;
					}
				}

				type A = { start: number, end: number, attrs: DecorationAttrs }
				
				function mapDecorations(ast: Token): { len: number, decorations: Array<A> } {
					const decorations = [];
					decorations.push(...extraDecorations(ast));
					if ("tokens" in ast) {
						decorations.push(...reduceDecorations(ast.tokens!, getOffset(ast.type)).decorations);
					}
					return {
						decorations,
						len: ast.raw.length
					}
				}

				function reduceDecorations(tokens: Array<Token>, startPos = 0) {
					return tokens.map(mapDecorations)
						.reduce(({ pos, decorations }, i) => ({
							pos: pos + i.len,
							decorations: [
								...decorations,
								...i.decorations.map((j: A) => ({ start: j.start + pos, end: j.end + pos, attrs: j.attrs })),
							],
						}), { pos: startPos, decorations: [] as Array<A> });
				}

				/*
				some nice colors from an old project
				  --background-1: #24262b;
  --background-2: #1e2024;
  --background-3: #191b1d;
  --background-4: #17181a;
  --foreground-1: #eae8efcc;
  --foreground-2: #eae8ef9f;
  --foreground-link: #b18cf3;
  --foreground-danger: #fa6261;
*/

				const reduced = reduceDecorations(md.lexer(state.doc.textContent), 1);
				return DecorationSet.create(state.doc, reduced.decorations.map(i => Decoration.inline(i.start, i.end, i.attrs)));
			},
			handlePaste(view, event, slice) {
				const files = event.clipboardData?.files ?? [];
				for (const file of files) props.onUpload?.(file);
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
				const parser = new globalThis.DOMParser();
				const tmp = parser.parseFromString(html, "text/html");

			  for (const node of tmp.querySelectorAll("script, form, svg, nav, footer, [hidden]:not([hidden=false]) [aria-hidden]:not([aria-hidden=false]) " + ["-ad-", "sponsor", "ad-break", "social", "sidebar", "comment"].map(i => `[class*=${i}], [id*=${i}]`).join(", "))) {
			    node.remove();
			  }

			  // FIXME: don't mangle whitespace
		    function walk(n: Node): string {
			    if (n.nodeType === Node.COMMENT_NODE) return "";
			    if (n.nodeType === Node.TEXT_NODE) return n.textContent ?? "";
			    
			    // TODO: tables
			    const c = [...n.childNodes];
			    switch (n.nodeName) {
			    	case "#document": case "HTML":
			      case "BODY": case "MAIN": case "ARTICLE": case "HEADER": case "SECTION":
			      case "DIV": case "TABLE": case "TBODY": case "THEAD": case "TR":
			      case "TURBO-FRAME": case "TASK-LISTS": // github
		      	case "X-HTML-IMPORT":
			        return c.map(walk).join("");
			      case "CENTER": case "SPAN": case "LI": case "TD": case "TH":
			        return c.map(walk).join("");
			      case "H1": return "\n\n# " + (n.textContent ?? "").trim() + "\n\n";
			      case "H2": return "\n\n## " + (n.textContent ?? "").trim() + "\n\n";
			      case "H3": return "\n\n### " + (n.textContent ?? "").trim() + "\n\n";
			      case "H4": return "\n\n#### " + (n.textContent ?? "").trim() + "\n\n";
			      case "H5": return "\n\n##### " + (n.textContent ?? "").trim() + "\n\n";
			      case "H6": return "\n\n###### " + (n.textContent ?? "").trim() + "\n\n";
			      case "P": return "\n\n" + c.map(walk).join("") + "\n\n";
			      case "B": case "BOLD": case "STRONG":
			        return `**${c.map(walk).join("")}**`;
			      case "EM": case "I":
			        return `*${c.map(walk).join("")}*`;
			      case "CODE":
			        return `\`${c.map(walk).join("")}\``;
			      case "UL":
			        return `\n${c.filter(i => i.nodeName === "LI").map(walk).map(i => `- ${i}`).join("\n")}\n`;
			      case "OL":
			        return `\n${c.filter(i => i.nodeName === "LI").map(walk).map((i, x) => `${x + 1}. ${i}`).join("\n")}\n`;
			      case "A": {
			      	const href = (n as Element).getAttribute("href");
			      	const text = c.map(walk).join("");
			      	if (!text) return "\n";
			      	if (!href) return text;
			        return `[${text}](${href})`;
			      }
			      case "PRE": {
			        const el = n as Element;
			        const lang = el.getAttribute("lang") ?? el.getAttribute("language") ?? el.getAttribute("class")?.match(/\b(lang|language)-(.+)\b/)?.[2] ?? "";
			        return `\n\n\`\`\`${lang}\n${n.textContent}\n\`\`\`\n\n`;
			      }
			      case "BLOCKQUOTE":
			        return `\n\n${c.map(walk).join("").split("\n").map(i => `> ${i}`).join("\n")}\n\n`;
			      // case "CITE":
			      //   return n.textContent;
			      default: {
			        return n.textContent ?? "";
			        // return `(??? ${n.nodeName} ???)`;
			      }
			    }
			  }

				const md = walk(tmp).replace(/\n{3,}/gm, "\n\n").replace(/^\n|\n$/g, "");
				console.log({ from: html, to: md });
				const p = document.createElement("pre");
				p.innerText = md;
				return p.outerHTML;
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
		createEffect(() => {
			console.log("new state", props.state)
			view.updateState(props.state);
		});
		onCleanup(() => view.destroy());
	});

	return (
		<div
			class="editor"
			classList={{ "disabled": props.disabled ?? false }}
			tabindex={0}
			ref={editorEl!}
			aria-placeholder={props.placeholder}
			role="textbox"
			aria-label="chat input"
		>
		</div>
	);
};

export default Editor;
