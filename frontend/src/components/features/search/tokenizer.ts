import { FILTER_NAMES } from "./filters.config";

// ---------------------------------------------------------------------------
// Tokenizer – single source of truth for parsing search query strings
// ---------------------------------------------------------------------------

export type Token =
	| { type: "text"; value: string; from: number; to: number }
	| {
			type: "filter";
			filterType: string;
			value: string;
			negated: boolean;
			from: number;
			to: number;
	  }
	| { type: "phrase"; value: string; from: number; to: number };

/**
 * Tokenize a search query string into a flat array of tokens.
 *
 * Handles:
 *   - `filter:value` and `-filter:value` (negated)
 *   - `"quoted phrases"`
 *   - plain text (including `-word` negation that is NOT a filter)
 */
export function tokenizeSearch(query: string): Token[] {
	const tokens: Token[] = [];
	let i = 0;

	while (i < query.length) {
		// --- quoted phrase ---
		if (query[i] === '"') {
			const end = query.indexOf('"', i + 1);
			if (end !== -1) {
				tokens.push({
					type: "phrase",
					value: query.slice(i + 1, end),
					from: i,
					to: end + 1,
				});
				i = end + 1;
				continue;
			}
			// Unterminated quote – treat rest as text
			tokens.push({
				type: "text",
				value: query.slice(i),
				from: i,
				to: query.length,
			});
			break;
		}

		// --- whitespace ---
		if (/\s/.test(query[i])) {
			const start = i;
			while (i < query.length && /\s/.test(query[i])) i++;
			tokens.push({
				type: "text",
				value: query.slice(start, i),
				from: start,
				to: i,
			});
			continue;
		}

		// --- potential filter (with optional leading `-`) ---
		const negated = query[i] === "-";
		const filterStart = negated ? i : i;
		const afterNegation = negated ? i + 1 : i;

		// Try to match `filterName:` at this position
		const matchedFilter = tryMatchFilter(query, afterNegation);
		if (matchedFilter) {
			const valueStart = matchedFilter.colonPos + 1;
			// Value runs until next whitespace
			let valueEnd = valueStart;
			while (valueEnd < query.length && !/\s/.test(query[valueEnd])) valueEnd++;

			const filterType = matchedFilter.filterName;
			const value = query.slice(valueStart, valueEnd);

			tokens.push({
				type: "filter",
				filterType,
				value,
				negated,
				from: filterStart,
				to: valueEnd,
			});
			i = valueEnd;
			continue;
		}

		// --- plain word (possibly negated with `-`) ---
		const start = i;
		// Consume non-whitespace
		while (i < query.length && !/\s/.test(query[i])) i++;

		const word = query.slice(start, i);
		// A `-word` that is NOT a filter is a negated text token
		if (negated && word.length > 1) {
			// Store as text with a marker for the compiler
			tokens.push({
				type: "text",
				value: word,
				from: start,
				to: i,
			});
		} else {
			tokens.push({
				type: "text",
				value: word,
				from: start,
				to: i,
			});
		}
	}

	return tokens;
}

/**
 * Attempt to match `filterName:` starting at `pos`.
 * Returns the filter name and the position of the colon, or null.
 */
function tryMatchFilter(
	query: string,
	pos: number,
): { filterName: string; colonPos: number } | null {
	for (const name of FILTER_NAMES) {
		if (
			query.slice(pos, pos + name.length).toLowerCase() === name &&
			query[pos + name.length] === ":"
		) {
			return { filterName: name, colonPos: pos + name.length };
		}
	}
	return null;
}
