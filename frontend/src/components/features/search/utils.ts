import type { Node } from "prosemirror-model";
import type { EditorState } from "prosemirror-state";
import type { User } from "sdk";
import { UUID } from "uuidv7";
import type { useUsers } from "@/api";
import type { ThreadT } from "@/types";
import { schema } from "./schema";

const RECENT_SEARCHES_KEY = "recent_searches";

export function getRecentSearches(): string[] {
	const stored = localStorage.getItem(RECENT_SEARCHES_KEY);
	if (!stored) return [];
	try {
		return JSON.parse(stored);
	} catch (_e) {
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

export function serializeToQuery(state: EditorState): string {
	let query = "";
	state.doc.forEach((node) => {
		node.forEach((inlineNode) => {
			if (inlineNode.isText) {
				query += inlineNode.text;
			} else {
				const type = inlineNode.type.name;
				const negated = inlineNode.attrs.negated ? "-" : "";
				if (type === "author" || type === "channel" || type === "mentions") {
					query += ` ${negated}${type}:${inlineNode.attrs.id} `;
				} else if (type === "before" || type === "after") {
					query += ` ${negated}${type}:${inlineNode.attrs.date} `;
				} else if (type === "has" || type === "pinned") {
					query += ` ${negated}${type}:${inlineNode.attrs.value} `;
				}
			}
		});
	});
	return query.trim().replace(/\s+/g, " ");
}

export function parseSearchQuery(query: string) {
	const tokens: {
		type: "filter" | "negated-filter";
		filterType: string;
		value: string;
		from: number;
		to: number;
		negated: boolean;
	}[] = [];

	const filterRegex =
		/(-?)(author|channel|before|after|has|pinned|mentions):(\S*)/g;
	let match;

	while ((match = filterRegex.exec(query)) !== null) {
		const isNegated = !!match[1];
		tokens.push({
			type: isNegated ? "negated-filter" : "filter",
			filterType: match[2],
			value: match[3],
			from: match.index,
			to: match.index + match[0].length,
			negated: isNegated,
		});
	}

	return tokens;
}

export function parseQueryToNodes(
	query: string,
	users2: ReturnType<typeof useUsers>,
	roomThreads: () => ThreadT[],
): Node[] {
	const nodes: Node[] = [];
	let textBuffer = "";

	const tokenRegex =
		/(-?)(author|channel|before|after|has|pinned|mentions):(\S*)|"([^"]*)"/g;
	let lastIndex = 0;
	let match;

	while ((match = tokenRegex.exec(query)) !== null) {
		const textBefore = query.slice(lastIndex, match.index);
		if (textBefore) textBuffer += textBefore;

		if (match[2]) {
			if (textBuffer) {
				nodes.push(schema.text(textBuffer));
				textBuffer = "";
			}

			const isNegated = !!match[1];
			const filterType = match[2];
			const value = match[3];

			if (filterType === "author") {
				const user = users2.cache.get(value) as User | undefined;
				if (user) {
					nodes.push(
						schema.nodes.author.create({
							id: user.id,
							name: user.name,
							negated: isNegated,
						}),
					);
				} else {
					textBuffer += `${isNegated ? "-" : ""}author:${value}`;
				}
			} else if (filterType === "channel") {
				const thread = roomThreads().find((t) => t.id === value);
				if (thread) {
					nodes.push(
						schema.nodes.channel.create({
							id: thread.id,
							name: thread.name,
							negated: isNegated,
						}),
					);
				} else {
					textBuffer += `${isNegated ? "-" : ""}channel:${value}`;
				}
			} else if (filterType === "before") {
				nodes.push(
					schema.nodes.before.create({ date: value, negated: isNegated }),
				);
			} else if (filterType === "after") {
				nodes.push(
					schema.nodes.after.create({ date: value, negated: isNegated }),
				);
			} else if (filterType === "has") {
				nodes.push(schema.nodes.has.create({ value, negated: isNegated }));
			} else if (filterType === "pinned") {
				nodes.push(schema.nodes.pinned.create({ value, negated: isNegated }));
			} else if (filterType === "mentions") {
				nodes.push(
					schema.nodes.mentions.create({
						id: value,
						name: value,
						negated: isNegated,
					}),
				);
			}
		} else if (match[4] !== undefined) {
			textBuffer += match[0];
		}

		lastIndex = tokenRegex.lastIndex;
	}

	if (lastIndex < query.length) textBuffer += query.slice(lastIndex);
	if (textBuffer) nodes.push(schema.text(textBuffer));

	return nodes;
}

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
		} else {
			date.setUTCHours(23, 59, 59, 999);
			const unixTsMs = date.getTime();
			const randA = 0xfff;
			const randBHi = 0x3fffffff;
			const randBLo = 0xffffffff;
			return UUID.fromFieldsV7(unixTsMs, randA, randBHi, randBLo).toString();
		}
	} catch (e) {
		console.error("Invalid date for search filter:", e);
		return undefined;
	}
}
