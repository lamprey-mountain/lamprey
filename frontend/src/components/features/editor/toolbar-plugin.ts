import { Plugin, PluginKey } from "prosemirror-state";
import type { FormattingToolbarContextT } from "../../../contexts/formatting-toolbar";

export const toolbarKey = new PluginKey("toolbar");

export function createToolbarPlugin(tb: FormattingToolbarContextT): Plugin {
	return new Plugin({
		key: toolbarKey,
		view(_editorView) {
			return {
				update(view, prevState) {
					if (!view.state.selection.eq(prevState.selection)) {
						const { state } = view;
						const { empty, from, to } = state.selection;
						if (empty || from === to) {
							tb.hideToolbar();
							return;
						}

						const coords = view.coordsAtPos(from);
						const endCoords = view.coordsAtPos(to);
						const top = Math.min(coords.top, endCoords.top);
						const left = coords.left;
						const width = Math.max(1, endCoords.left - coords.left);
						const height = Math.max(
							coords.bottom - coords.top,
							endCoords.bottom - endCoords.top,
						);

						tb.showToolbar({
							getBoundingClientRect() {
								return {
									x: left,
									y: top,
									width,
									height,
									left,
									top,
									right: left + width,
									bottom: top + height,
								};
							},
						});
					}
				},
			};
		},
	});
}
