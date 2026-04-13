import type { Node } from "prosemirror-model";
import type { EditorState } from "prosemirror-state";
import type { User } from "sdk";
import { UUID } from "uuidv7";
import type { useUsers } from "@/api";
import type { ThreadT } from "@/types";
import { SEARCH_FILTERS, type SearchContext } from "./filters.config";
import { schema } from "./schema";
import { type Token, tokenizeSearch } from "./tokenizer";
import type { LabelPart } from "./types";

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
// parseQueryToNodes – uses the registry to create PM nodes from a string
// ---------------------------------------------------------------------------

export function parseQueryToNodes(query: string, ctx: SearchContext): Node[] {
	const nodes: Node[] = [];
	let textBuffer = "";
	const tokens = tokenizeSearch(query);
	let lastTo = 0;

	for (const token of tokens) {
		if (token.from > lastTo) textBuffer += query.slice(lastTo, token.from);
		lastTo = token.to;

		if (token.type === "phrase" || token.type === "text") {
			textBuffer += token.value;
			continue;
		}

		if (textBuffer) {
			nodes.push(schema.text(textBuffer));
			textBuffer = "";
		}

		const def = SEARCH_FILTERS[token.filterType];
		if (!def) {
			textBuffer += query.slice(token.from, token.to);
			continue;
		}

		let name = token.value;
		if (def.resolveDisplayData) {
			const resolved = def.resolveDisplayData(token.value, ctx);
			if (resolved.name) name = resolved.name;
		}

		nodes.push(
			def.toPMNode({
				type: token.filterType,
				value: token.value,
				name: name,
				negated: token.negated ?? false,
			}),
		);
	}

	if (lastTo < query.length) textBuffer += query.slice(lastTo);
	if (textBuffer) nodes.push(schema.text(textBuffer));

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

export function formatRecentSearch(
	query: string,
	ctx: SearchContext,
): LabelPart[] {
	const tokens = tokenizeSearch(query);
	const parts: LabelPart[] = [];
	let lastTo = 0;

	for (const token of tokens) {
		// 1. Push plain text between tokens
		if (token.from > lastTo) {
			parts.push(query.slice(lastTo, token.from));
		}
		lastTo = token.to;

		// 2. Handle Text/Phrases
		if (token.type !== "filter") {
			parts.push(token.value);
			continue;
		}

		// 3. Handle Filters cleanly using the registry
		const def = SEARCH_FILTERS[token.filterType];
		if (def && def.resolveDisplayData) {
			const resolved = def.resolveDisplayData(token.value, ctx);
			parts.push({
				type: token.filterType,
				value: resolved.name ?? token.value,
				user: resolved.user,
				channel: resolved.channel,
				negated: token.negated,
				parts: [], // Triggers FilterChipUI
			});
		} else {
			// Fallback for simple filters like has:image
			parts.push({
				type: token.filterType,
				value: token.value,
				negated: token.negated,
				parts: [],
			});
		}
	}

	if (lastTo < query.length) parts.push(query.slice(lastTo));
	return parts;
}
