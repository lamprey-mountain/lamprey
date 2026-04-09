import { type EditorState, Plugin, PluginKey } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";
import { getActiveFilterAtCursor, tokenizeSearch } from "./tokenizer";

// ---------------------------------------------------------------------------
// Autocomplete plugin state
// ---------------------------------------------------------------------------

export interface AutocompleteState {
	active: boolean;
	filterType: string;
	query: string;
	negated: boolean;
}

export const autocompleteKey = new PluginKey<AutocompleteState>("autocomplete");

/**
 * Get the active filter context from a plain text string (e.g. the text
 * before the cursor in the current paragraph node).
 */
export function getFilterFromSelection(state: EditorState): {
	type: string;
	query: string;
	negated?: boolean;
} | null {
	const { selection } = state;
	if (!selection.empty) return null;

	const $pos = state.doc.resolve(selection.from);
	const nodeBefore = $pos.nodeBefore;

	// at the start of doc or paragraph
	if (!nodeBefore) return { type: "filter", query: "" };

	// after an atom node
	if (!nodeBefore.isText) return { type: "filter", query: "" };

	const textBeforeCursor = nodeBefore.text!;

	// inside a filter value
	const active = getActiveFilterAtCursor(
		textBeforeCursor,
		textBeforeCursor.length,
	);
	if (active) {
		return {
			type: active.filterType,
			query: active.value,
			negated: active.negated,
		};
	}

	// trailing space
	if (textBeforeCursor.match(/\s$/)) {
		return { type: "filter", query: "" };
	}

	// typing a word that isnt a filter yet
	const wordMatch = textBeforeCursor.match(/(\S+)$/);
	if (wordMatch) {
		const word = wordMatch[1];
		const negated = word.startsWith("-");
		const cleanWord = negated ? word.slice(1) : word;
		if (cleanWord.includes(":")) return null;
		return { type: "filter", query: cleanWord, negated };
	}

	return { type: "filter", query: "" };
}

// ---------------------------------------------------------------------------
// Syntax highlighting plugin – uses the tokenizer
// ---------------------------------------------------------------------------

export function syntaxHighlightingPlugin() {
	return new Plugin({
		props: {
			decorations(state) {
				const decorations: Decoration[] = [];
				state.doc.descendants((node, pos) => {
					// Filter atoms
					if (node.type.name !== "text" && node.isAtom) {
						decorations.push(
							Decoration.inline(pos, pos + node.nodeSize, {
								class: `filter-atom filter-${node.type.name}`,
							}),
						);
						return false;
					}

					if (!node.isText) return;

					const text = node.text!;
					const tokens = tokenizeSearch(text);

					for (const token of tokens) {
						if (token.type === "filter") {
							const from = pos + token.from;
							const to = pos + token.to;
							const negatedClass = token.negated ? " filter-negated" : "";
							decorations.push(
								Decoration.inline(from, to, {
									class: `filter-token filter-${token.filterType}${negatedClass}`,
								}),
							);
						} else if (token.type === "phrase") {
							const from = pos + token.from;
							const to = pos + token.to;
							decorations.push(
								Decoration.inline(from, to, { class: "filter-phrase" }),
							);
							decorations.push(
								Decoration.inline(from, from + 1, { class: "syn" }),
							);
							if (token.value.length > 0) {
								decorations.push(
									Decoration.inline(to - 1, to, { class: "syn" }),
								);
							}
						}
					}
				});
				return DecorationSet.create(state.doc, decorations);
			},
		},
	});
}

// ---------------------------------------------------------------------------
// Autocomplete plugin
// ---------------------------------------------------------------------------

export function autocompletePlugin(
	setFilter: (
		filter: { type: string; query: string; negated?: boolean } | null,
	) => void,
) {
	return new Plugin({
		key: autocompleteKey,
		state: {
			init: () => ({
				active: false,
				filterType: "filter",
				query: "",
				negated: false,
			}),
			apply(tr, _old, oldState, newState) {
				if (tr.getMeta("skipAutocomplete")) {
					return {
						active: false,
						filterType: "filter",
						query: "",
						negated: false,
					};
				}
				if (!tr.docChanged && !tr.selectionSet) {
					const prev = autocompleteKey.getState(oldState);
					return (
						prev ?? {
							active: false,
							filterType: "filter",
							query: "",
							negated: false,
						}
					);
				}

				const filterInfo = getFilterFromSelection(newState);
				const isActive = filterInfo !== null;
				const next = filterInfo ?? {
					type: "filter",
					query: "",
					negated: false,
				};

				return {
					active: isActive,
					filterType: next.type,
					query: next.query ?? "",
					negated: next.negated ?? false,
				};
			},
		},
		view() {
			return {
				update(view, prevState) {
					const prev = autocompleteKey.getState(prevState);
					const curr = autocompleteKey.getState(view.state);

					// Only update the SolidJS signal if the autocomplete state actually changed
					if (
						prev?.active !== curr?.active ||
						prev?.query !== curr?.query ||
						prev?.filterType !== curr?.filterType ||
						prev?.negated !== curr?.negated
					) {
						if (curr?.active) {
							setFilter({
								type: curr.filterType,
								query: curr.query,
								negated: curr.negated,
							});
						} else {
							setFilter(null);
						}
					}
				},
			};
		},
		props: {
			handleKeyDown(_view, event) {
				// Let SolidJS / parent plugin handle navigation for now.
				// Future: move ArrowDown/ArrowUp/Enter state entirely here.
				if (event.key === "Escape") {
					setFilter(null);
					return true;
				}
				return false;
			},
		},
	});
}
