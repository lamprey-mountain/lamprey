import { Parser } from "@lamprey/markdown";
import { DOMParser } from "prosemirror-model";
import { Plugin, PluginKey, TextSelection } from "prosemirror-state";
import { initTurndownService } from "@/lib/markdown/turndown";
import { serializeToEditorHTML } from "./editor-html.ts";
import { schema } from "./schema.ts";

export const pastePluginKey = new PluginKey("paste");

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

				const html = event.clipboardData?.getData("text/html");
				const plainText = event.clipboardData?.getData("text/plain");

				// TODO: return false if this is prosemirror data?

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

				const parser = new Parser();
				const parsed = parser.parse(str);
				const div = document.createElement("div");
				div.innerHTML = serializeToEditorHTML(parsed.ast());
				const parsedSlice = DOMParser.fromSchema(schema).parseSlice(div);

				view.dispatch(
					view.state.tr
						.replaceSelection(parsedSlice)
						.scrollIntoView()
						.setMeta("paste", true)
						.setMeta("uiEvent", "paste"),
				);
				return true;
			},
		},
	});
}
