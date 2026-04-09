import type { Node } from "prosemirror-model";
import type { EditorState } from "prosemirror-state";
import type { User } from "sdk";
import { UUID } from "uuidv7";
import type { useUsers } from "@/api";
import type { ThreadT } from "@/types";
import { SEARCH_FILTERS } from "./filters.config";
import { schema } from "./schema";
import { type Token, tokenizeSearch } from "./tokenizer";

const RECENT_SEARCHES_KEY = "recent_searches";

// ---------------------------------------------------------------------------
// Recent searches (unchanged – this is fine)
// ---------------------------------------------------------------------------

export function getRecentSearches(): string[] {
	const stored = localStorage.getItem(RECENT_SEARCHES_KEY);
	if (!stored) return [];
	try {
		return JSON.parse(stored);
	} catch {
		return [];
	}
}

export function addRecentSearch(query: string) {
	if (!query.trim()) return;
	const normalizedQuery = query.trim().replace(/\s+/g, " ");
	let searches = getRecentSearches();
	searches = [
		normalizedQuery,
		...searches.filter((s) => s !== normalizedQuery),
	].slice(0, 10);
	localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(searches));
}

// ---------------------------------------------------------------------------
// Serialize PM doc → query string (uses the filter registry)
// ---------------------------------------------------------------------------

export function serializeToQuery(state: EditorState): string {
	let query = "";
	state.doc.forEach((node) => {
		node.forEach((inlineNode) => {
			if (inlineNode.isText) {
				query += inlineNode.text;
			} else {
				const def = SEARCH_FILTERS[inlineNode.type.name];
				if (!def) return;
				const negated = inlineNode.attrs.negated ? "-" : "";
				if (def.hasNameAttr) {
					query += ` ${negated}${inlineNode.type.name}:${inlineNode.attrs.id} `;
				} else if (def.valueType === "date") {
					query += ` ${negated}${inlineNode.type.name}:${inlineNode.attrs.date} `;
				} else {
					query += ` ${negated}${inlineNode.type.name}:${inlineNode.attrs.value} `;
				}
			}
		});
	});
	return query.trim().replace(/\s+/g, " ");
}

// ---------------------------------------------------------------------------
// parseSearchQuery – now a thin wrapper around the tokenizer
// ---------------------------------------------------------------------------

export function parseSearchQuery(query: string) {
	const tokens = tokenizeSearch(query);
	return tokens
		.filter((t): t is Token & { type: "filter" } => t.type === "filter")
		.map((t) => ({
			type: "filter" as const,
			filterType: t.filterType,
			value: t.value,
			from: t.from,
			to: t.to,
			negated: t.negated,
		}));
}

// ---------------------------------------------------------------------------
// parseQueryToNodes – uses the registry to create PM nodes from a string
// ---------------------------------------------------------------------------

export function parseQueryToNodes(
	query: string,
	users: ReturnType<typeof useUsers>,
	roomThreads: () => ThreadT[],
): Node[] {
	const nodes: Node[] = [];
	let textBuffer = "";

	const tokens = tokenizeSearch(query);
	let lastTo = 0;

	for (const token of tokens) {
		// Text between tokens
		if (token.from > lastTo) {
			textBuffer += query.slice(lastTo, token.from);
		}
		lastTo = token.to;

		if (token.type === "phrase") {
			// Preserve quoted phrases as plain text
			textBuffer += token.value;
			continue;
		}

		if (token.type === "text") {
			textBuffer += token.value;
			continue;
		}

		// Filter token – flush text buffer, then create node
		if (textBuffer) {
			nodes.push(schema.text(textBuffer));
			textBuffer = "";
		}

		const def = SEARCH_FILTERS[token.filterType];
		if (!def) {
			// Unknown filter – keep as text
			textBuffer += query.slice(token.from, token.to);
			continue;
		}

		// For id-type filters, look up the entity to get the display name
		if (def.valueType === "id" && def.hasNameAttr) {
			if (token.filterType === "author") {
				const user = users.cache.get(token.value) as User | undefined;
				if (user) {
					nodes.push(
						schema.nodes.author.create({
							id: user.id,
							name: user.name,
							negated: token.negated,
						}),
					);
				} else {
					textBuffer += query.slice(token.from, token.to);
				}
				continue;
			}
			if (token.filterType === "channel") {
				const thread = roomThreads().find((t) => t.id === token.value);
				if (thread) {
					nodes.push(
						schema.nodes.channel.create({
							id: thread.id,
							name: thread.name,
							negated: token.negated,
						}),
					);
				} else {
					textBuffer += query.slice(token.from, token.to);
				}
				continue;
			}
			if (token.filterType === "mentions") {
				nodes.push(
					schema.nodes.mentions.create({
						id: token.value,
						name: token.value,
						negated: token.negated,
					}),
				);
				continue;
			}
		}

		// Date, value, or id without name lookup
		if (def.valueType === "date") {
			nodes.push(
				schema.nodes[token.filterType].create({
					date: token.value,
					negated: token.negated,
				}),
			);
		} else if (def.valueType === "value") {
			nodes.push(
				schema.nodes[token.filterType].create({
					value: token.value,
					negated: token.negated,
				}),
			);
		} else if (def.valueType === "id" && !def.hasNameAttr) {
			// Fallback for id types that don't need name resolution
			nodes.push(
				schema.nodes[token.filterType].create({
					id: token.value,
					negated: token.negated,
				}),
			);
		} else {
			// Fallback: keep as text
			textBuffer += query.slice(token.from, token.to);
		}
	}

	// Remaining text after last token
	if (lastTo < query.length) {
		textBuffer += query.slice(lastTo);
	}
	if (textBuffer) {
		nodes.push(schema.text(textBuffer));
	}

	return nodes;
}

// ---------------------------------------------------------------------------
// dateToBoundaryUUID (unchanged)
// ---------------------------------------------------------------------------

export function dateToBoundaryUUID(
	dateString: string,
	boundary: "start" | "end",
): string | undefined {
	try {
		const date = new Date(dateString);
		if (Number.isNaN(date.getTime())) return undefined;

		if (boundary === "start") {
			date.setUTCHours(0, 0, 0, 0);
			const unixTsMs = date.getTime();
			return UUID.fromFieldsV7(unixTsMs, 0, 0, 0).toString();
		}
		date.setUTCHours(23, 59, 59, 999);
		const unixTsMs = date.getTime();
		const randA = 0xfff;
		const randBHi = 0x3fffffff;
		const randBLo = 0xffffffff;
		return UUID.fromFieldsV7(unixTsMs, randA, randBHi, randBLo).toString();
	} catch (e) {
		console.error("Invalid date for search filter:", e);
		return undefined;
	}
}
