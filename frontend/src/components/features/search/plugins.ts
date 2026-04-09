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

	if (!nodeBefore) return { type: "filter", query: "" };
	if (!nodeBefore.isText) return null;

	const textBeforeCursor = nodeBefore.text!;

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

	// If text ends with whitespace or is empty → generic filter context
	if (textBeforeCursor.match(/\s$/) || textBeforeCursor === "") {
		return { type: "filter", query: "" };
	}

	// If cursor is in the middle of a word that isn't a filter → generic
	const wordMatch = textBeforeCursor.match(/(\S+)$/);
	if (wordMatch) {
		const word = wordMatch[1];
		const cleanWord = word.startsWith("-") ? word.slice(1) : word;
		if (cleanWord.includes(":")) return null;
		return { type: "filter", query: cleanWord, negated: word.startsWith("-") };
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
