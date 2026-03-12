import { Plugin, PluginKey } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";

export type DiffMark =
	| { from: number; to: number; type: "insertion" }
	| { pos: number; type: "deletion"; text: string };

export const diffPluginKey = new PluginKey("diff");

export function createDiffPlugin(
	_getDiffMarks: () => DiffMark[],
): Plugin {
	return new Plugin({
		key: diffPluginKey,
		state: {
			init() {
				return DecorationSet.empty;
			},
			apply(tr, value, _oldState, newState) {
				value = value.map(tr.mapping, tr.doc);
				const meta = tr.getMeta(diffPluginKey);
				if (meta?.marks) {
					const marks: DiffMark[] = meta.marks;

					const decorations = marks.map((mark) => {
						if (mark.type === "insertion") {
							// Clamp positions to valid range
							const from = Math.min(mark.from, newState.doc.content.size);
							const to = Math.min(mark.to, newState.doc.content.size);
							return Decoration.inline(from, to, {
								class: "diff-insertion",
							});
						} else {
							const pos = Math.min(mark.pos, newState.doc.content.size);
							return Decoration.widget(pos, () => {
								const dom = document.createElement("span");
								dom.className = "diff-deletion";
								dom.textContent = mark.text;
								return dom;
							});
						}
					});
					try {
						return DecorationSet.create(newState.doc, decorations);
					} catch (e) {
						console.error("[diff-plugin] failed to create decorations:", e);
						return value;
					}
				}
				return value;
			},
		},
		props: {
			decorations(state) {
				return this.getState(state);
			},
		},
	});
}

export function setDiffMarks(
	tr: any,
	marks: DiffMark[],
): any {
	return tr.setMeta(diffPluginKey, { marks });
}
