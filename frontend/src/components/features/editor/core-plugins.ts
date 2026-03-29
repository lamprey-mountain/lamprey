import { Plugin, PluginKey, TextSelection } from "prosemirror-state";
import { initTurndownService } from "../../../turndown.ts";
import { convertEmojiInText } from "./emoji-plugin.ts";
import { schema } from "./schema.ts";
import { serializeToMarkdown } from "./serializer.ts";

export const pastePluginKey = new PluginKey("paste");
export const submitPluginKey = new PluginKey("submit");

const turndown = initTurndownService();

export function createPastePlugin() {
	return new Plugin({
		key: pastePluginKey,
		state: {
			init() {
				return { onUpload: null };
			},
			apply(tr, value) {
				const meta = tr.getMeta(pastePluginKey);
				if (meta) return { ...value, ...meta };
				return value;
			},
		},
		props: {
			handlePaste(view, event, slice) {
				const { onUpload } = pastePluginKey.getState(view.state) || {};
				const files = Array.from(event.clipboardData?.files ?? []);

				if (files.length) {
					for (const file of files) onUpload?.(file);
					return true;
				}

				const isInternal = event.clipboardData?.types.includes(
					"application/x-prosemirror-slice",
				);
				if (isInternal) return false;

				const html = event.clipboardData?.getData("text/html");
				const plainText = event.clipboardData?.getData("text/plain");

				const str = html
					? turndown.turndown(html)
					: (plainText ??
						slice.content.textBetween(0, slice.content.size, "\n"));

				const tr = view.state.tr;
				if (
					!tr.selection.empty &&
					/^(https?:\/\/|mailto:)\S+$/i.test(str.trim())
				) {
					const url = str.trim();
					const { from, to } = tr.selection;
					tr.insertText(`](${url})`, to);
					tr.insertText("[", from);
					tr.setSelection(TextSelection.create(tr.doc, tr.mapping.map(to)));
					view.dispatch(
						tr
							.scrollIntoView()
							.setMeta("paste", true)
							.setMeta("uiEvent", "paste"),
					);
					return true;
				}

				const { content, hasEmoji } = convertEmojiInText(schema, str);

				if (hasEmoji) {
					const { from, to } = view.state.selection;
					view.dispatch(
						view.state.tr
							.replaceWith(from, to, content)
							.scrollIntoView()
							.setMeta("paste", true),
					);
					return true;
				}

				view.dispatch(
					view.state.tr
						.replaceSelectionWith(schema.text(str))
						.scrollIntoView()
						.setMeta("paste", true),
				);
				return true;
			},
		},
	});
}

function isInsideCodeBlock(state: any): boolean {
	const { $from } = state.selection;
	for (let d = $from.depth; d > 0; d--) {
		if ($from.node(d).type.name === "code_block") return true;
	}
	return false;
}

export function createSubmitPlugin() {
	return new Plugin({
		key: submitPluginKey,
		state: {
			init() {
				return { onSubmit: null, submitOnEnter: true };
			},
			apply(tr, value) {
				const meta = tr.getMeta(submitPluginKey);
				if (meta) return { ...value, ...meta };
				return value;
			},
		},
		props: {
			handleKeyDown(view, event) {
				const { onSubmit, submitOnEnter } =
					submitPluginKey.getState(view.state) || {};

				const submitCommand = (viewState: any, dispatch: any) => {
					const res = onSubmit?.(serializeToMarkdown(viewState.doc).trim());
					if (res instanceof Promise) {
						res.then((shouldClear) => {
							if (shouldClear) {
								view.dispatch(
									view.state.tr.deleteRange(0, view.state.doc.nodeSize - 2),
								);
							}
						});
					} else if (res) {
						dispatch?.(viewState.tr.deleteRange(0, viewState.doc.nodeSize - 2));
					}
					return true;
				};

				if (event.key === "Enter" && !event.shiftKey) {
					// Don't auto-submit inside codeblocks
					if (isInsideCodeBlock(view.state)) {
						view.dispatch(view.state.tr.insertText("\n").scrollIntoView());
						return true;
					}

					if (submitOnEnter ?? true) {
						return submitCommand(view.state, view.dispatch);
					} else {
						view.dispatch(view.state.tr.insertText("\n").scrollIntoView());
						return true;
					}
				}

				if (event.key === "Enter" && event.ctrlKey) {
					return submitCommand(view.state, view.dispatch);
				}

				return false;
			},
		},
	});
}
