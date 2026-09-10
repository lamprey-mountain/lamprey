import { Plugin, PluginKey } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";

export const placeholderPluginKey = new PluginKey<string>("placeholder");

export function createPlaceholderPlugin() {
	return new Plugin<string>({
		key: placeholderPluginKey,
		state: {
			init: () => "",
			apply(tr, prev) {
				const meta = tr.getMeta(placeholderPluginKey);
				if (meta !== undefined) return meta;
				return prev;
			},
		},
		props: {
			decorations(state) {
				const text = placeholderPluginKey.getState(state);
				if (!text) return DecorationSet.empty;
				const isEmpty = !state.doc.firstChild?.content.size;
				if (!isEmpty) return DecorationSet.empty;

				const widget = Decoration.widget(0, () => {
					const span = document.createElement("div");
					span.className = "placeholder";
					span.textContent = text;
					return span;
				});
				return DecorationSet.create(state.doc, [widget]);
			},
		},
	});
}
