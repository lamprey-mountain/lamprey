import { type EditorState, Plugin } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";
import { parseSearchQuery } from "./utils";

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

	const tokens = parseSearchQuery(textBeforeCursor);
	const lastToken = tokens[tokens.length - 1];
	if (lastToken && lastToken.to === textBeforeCursor.length) {
		return {
			type: lastToken.filterType,
			query: lastToken.value,
			negated: lastToken.negated,
		};
	}

	const partialFilterMatch = textBeforeCursor.match(
		/(-?)(author|channel|before|after|has|pinned|mentions):$/,
	);
	if (partialFilterMatch) {
		return {
			type: partialFilterMatch[2],
			query: "",
			negated: !!partialFilterMatch[1],
		};
	}

	if (textBeforeCursor.match(/\s$/)) return { type: "filter", query: "" };

	const wordMatch = textBeforeCursor.match(/(\S+)$/);
	if (wordMatch) {
		const word = wordMatch[1];
		const cleanWord = word.startsWith("-") ? word.slice(1) : word;
		if (cleanWord.includes(":")) return null;

		return { type: "filter", query: cleanWord, negated: word.startsWith("-") };
	}

	return { type: "filter", query: "" };
}

export function syntaxHighlightingPlugin() {
	return new Plugin({
		props: {
			decorations(state) {
				const decorations: Decoration[] = [];
				state.doc.descendants((node, pos) => {
					if (node.type.name !== "text" && node.isAtom) {
						decorations.push(
							Decoration.inline(pos, pos + node.nodeSize, {
								class: `filter-atom filter-${node.type.name}`,
							}),
						);
						return false;
					}

					if (node.isText) {
						const text = node.text!;

						const tokens = parseSearchQuery(text);
						for (const token of tokens) {
							const from = pos + token.from;
							const to = pos + token.to;
							const negatedClass = token.negated ? " filter-negated" : "";
							decorations.push(
								Decoration.inline(from, to, {
									class: `filter-token filter-${token.filterType}${negatedClass}`,
								}),
							);
						}

						const phraseRegex = /"([^"]*)"/g;
						let match;
						while ((match = phraseRegex.exec(text))) {
							const from = pos + match.index;
							const to = from + match[0].length;
							decorations.push(
								Decoration.inline(from, to, { class: "filter-phrase" }),
							);
							decorations.push(
								Decoration.inline(from, from + 1, { class: "syn" }),
							);
							if (match[0].length > 1) {
								decorations.push(
									Decoration.inline(to - 1, to, { class: "syn" }),
								);
							}
						}

						const negationRegex = /(^|\s)-\S+/g;
						while ((match = negationRegex.exec(text))) {
							const from = pos + match.index + (match[1]?.length ?? 0);
							const to = from + match[0].length - (match[1]?.length ?? 0);
							const negatedText = match[0].trimStart();

							if (
								!negatedText.match(
									/^-(author|channel|before|after|has|pinned|mentions):/,
								)
							) {
								decorations.push(
									Decoration.inline(from, to, { class: "filter-negation" }),
								);
								decorations.push(
									Decoration.inline(from, from + 1, { class: "syn" }),
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

export function autocompletePlugin(
	setFilter: (
		filter: { type: string; query: string; negated?: boolean } | null,
	) => void,
) {
	return new Plugin({
		state: {
			init: () => null,
			apply: (tr, value, _oldState, newState) => {
				if (tr.getMeta("skipAutocomplete")) {
					setFilter(null);
					return null;
				}
				if (!tr.docChanged && !tr.selectionSet) return value;

				setFilter(getFilterFromSelection(newState));
				return null;
			},
		},
	});
}
