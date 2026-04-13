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
	from: number;
	to: number;
}

export const autocompleteKey = new PluginKey<AutocompleteState>("autocomplete");

export type ActiveFilter = {
	type: string;
	query: string;
	negated?: boolean;
	from: number;
	to: number;
};

/**
 * Get the active filter context from a plain text string (e.g. the text
 * before the cursor in the current paragraph node).
 */
export function getFilterFromSelection(
	state: EditorState,
): ActiveFilter | null {
	const { selection } = state;
	if (!selection.empty) return null;

	const $pos = selection.$from;
	const textBefore = $pos.nodeBefore?.isText ? $pos.nodeBefore.text! : "";

	// 'offset' is the absolute doc position where the current text node starts
	const offset = $pos.pos - textBefore.length;

	const tokens = tokenizeSearch(textBefore);
	const lastToken = tokens[tokens.length - 1];

	if (!lastToken) {
		return {
			type: "filter",
			query: "",
			negated: false,
			from: $pos.pos,
			to: $pos.pos,
		};
	}

	// 1. Check if we are inside a specific filter value (e.g., author:jo|)
	if (lastToken.type === "filter") {
		return {
			type: lastToken.filterType,
			query: lastToken.value,
			negated: lastToken.negated,
			from: offset + lastToken.from,
			to: offset + lastToken.to,
		};
	}

	// 2. Trailing space means we are starting a fresh generic search
	if (!textBefore || textBefore.match(/\s$/)) {
		return {
			type: "filter",
			query: "",
			negated: false,
			from: $pos.pos,
			to: $pos.pos,
		};
	}

	// 3. We are typing a word that might become a filter (e.g. -aut|)
	const wordMatch = textBefore.match(/(\S+)$/);
	if (wordMatch) {
		const word = wordMatch[1];

		// If it already contains a colon but wasn't caught by getActiveFilterAtCursor, it's invalid
		if (word.includes(":") && !word.endsWith(":")) return null;

		const negated = word.startsWith("-");
		const query = (negated ? word.slice(1) : word).replace(":", "");

		return {
			type: "filter",
			query,
			negated,
			from: $pos.pos - word.length,
			to: $pos.pos,
		};
	}

	return {
		type: "filter",
		query: "",
		negated: false,
		from: $pos.pos,
		to: $pos.pos,
	};
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
		filter: {
			type: string;
			query: string;
			negated?: boolean;
			from: number;
			to: number;
		} | null,
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
				from: 0,
				to: 0,
			}),
			apply(tr, _old, oldState, newState) {
				if (tr.getMeta("skipAutocomplete")) {
					return {
						active: false,
						filterType: "filter",
						query: "",
						negated: false,
						from: 0,
						to: 0,
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
							from: 0,
							to: 0,
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
					from: filterInfo?.from ?? 0,
					to: filterInfo?.to ?? 0,
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
								from: curr.from,
								to: curr.to,
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
