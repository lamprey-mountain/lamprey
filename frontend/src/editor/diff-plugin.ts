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
					const decorations: Decoration[] = [];
					const maxPos = newState.doc.content.size - 1;

					for (const mark of marks) {
						if (mark.type === "insertion") {
							let from = Math.max(0, Math.min(mark.from, maxPos));
							const to = Math.max(0, Math.min(mark.to, maxPos));
							if (from >= to) continue;

							while (from < to) {
								const node = newState.doc.nodeAt(from);
								if (!node) break;

								const nodeEnd = Math.min(from + node.nodeSize, to);
								decorations.push(Decoration.inline(from, nodeEnd, {
									class: "diff-insertion",
								}));
								from = nodeEnd;
							}
						} else {
							const pos = Math.max(0, Math.min(mark.pos, maxPos));
							decorations.push(Decoration.widget(pos, () => {
								const dom = document.createElement("span");
								dom.className = "diff-deletion";
								dom.textContent = mark.text;
								return dom;
							}));
						}
					}

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
