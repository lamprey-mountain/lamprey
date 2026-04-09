import type { EditorState } from "prosemirror-state";
import { UUID } from "uuidv7";
import type { RoomT, ThreadT } from "@/types";
import { type FilterASTNode, SEARCH_FILTERS } from "./filters.config";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export interface SearchRequestBody {
	query?: string;
	sort_order: "asc" | "desc";
	sort_field: "Created";
	limit: number;
}

/**
 * Given a ProseMirror editor state, produce the backend search request body.
 * Pure function – no framework dependencies, easy to unit-test.
 */
export function buildBackendSearchBody(
	state: EditorState,
	context: {
		channel?: ThreadT;
		room?: RoomT;
	},
): SearchRequestBody {
	const ast = extractAST(state);
	const queryParts = compileAST(ast, context);

	return {
		query: queryParts.length > 0 ? queryParts.join(" ") : undefined,
		sort_order: "desc" as const,
		sort_field: "Created" as const,
		limit: 100,
	};
}

// ---------------------------------------------------------------------------
// Intermediate AST
// ---------------------------------------------------------------------------

interface SearchAST {
	textQueries: string[];
	negatedTextQueries: string[];
	filters: FilterASTNode[];
	beforeDate?: string;
	afterDate?: string;
}

/**
 * Walk the ProseMirror document and produce a SearchAST.
 */
function extractAST(state: EditorState): SearchAST {
	const ast: SearchAST = {
		textQueries: [],
		negatedTextQueries: [],
		filters: [],
	};

	state.doc.forEach((blockNode) => {
		blockNode.forEach((inlineNode) => {
			if (inlineNode.isText) {
				const text = inlineNode.text?.trim();
				if (!text) return;
				const words = text.split(/\s+/);
				for (const word of words) {
					if (!word) continue;
					if (word.startsWith("-") && word.length > 1) {
						ast.negatedTextQueries.push(word.slice(1));
					} else {
						ast.textQueries.push(word);
					}
				}
			} else {
				const def = SEARCH_FILTERS[inlineNode.type.name];
				if (!def) return;

				const filterNode = def.toAST(inlineNode);
				ast.filters.push(filterNode);

				if (filterNode.type === "before") {
					ast.beforeDate = filterNode.value;
				}
				if (filterNode.type === "after") {
					ast.afterDate = filterNode.value;
				}
			}
		});
	});

	return ast;
}

/**
 * Compile a SearchAST into the array of backend query string fragments.
 */
function compileAST(
	ast: SearchAST,
	context: { channel?: ThreadT; room?: RoomT },
): string[] {
	const parts: string[] = [];

	// --- text queries ---
	if (ast.textQueries.length) {
		parts.push(`+(${ast.textQueries.join(" ")})`);
	}
	if (ast.negatedTextQueries.length) {
		parts.push(`-(${ast.negatedTextQueries.join(" ")})`);
	}

	// --- filter queries (delegated to the registry) ---
	for (const filter of ast.filters) {
		const def = SEARCH_FILTERS[filter.type];
		if (!def) continue;

		// Handled specially below
		if (
			filter.type === "before" ||
			filter.type === "after" ||
			filter.type === "channel" ||
			filter.type === "author" ||
			filter.type === "pinned"
		)
			continue;

		const fragments = def.toBackendQuery(filter);
		parts.push(...fragments);
	}

	// --- scope (channel / room) ---
	parts.push(...compileScope(context, ast));

	// --- author grouping (OR logic) ---
	const positiveAuthors = ast.filters.filter(
		(f) => f.type === "author" && !f.negated,
	);
	if (positiveAuthors.length) {
		parts.push(
			`+author_id: IN [${positiveAuthors.map((f) => f.value).join(" ")}]`,
		);
	}
	const negativeAuthors = ast.filters.filter(
		(f) => f.type === "author" && f.negated,
	);
	if (negativeAuthors.length) {
		parts.push(
			`-author_id: IN [${negativeAuthors.map((f) => f.value).join(" ")}]`,
		);
	}

	// --- date range ---
	if (ast.beforeDate || ast.afterDate) {
		parts.push(...compileDateRange(ast.beforeDate, ast.afterDate));
	}

	return parts;
}

/** Compile room/channel scoping fragments */
function compileScope(
	context: { channel?: ThreadT; room?: RoomT },
	ast: SearchAST,
): string[] {
	const parts: string[] = [];
	const channelFilters = ast.filters.filter((f) => f.type === "channel");
	const negatedChannelFilters = ast.filters.filter(
		(f) => f.type === "channel" && f.negated,
	);
	const positiveChannels = channelFilters.filter((f) => !f.negated);

	if (context.channel) {
		const ch = context.channel;
		if (ch.type === "Dm" || ch.type === "Gdm") {
			parts.push(`+channel_id:${ch.id}`);
		} else if (positiveChannels.length) {
			parts.push(
				`+channel_id: IN [${positiveChannels.map((f) => f.value).join(" ")}]`,
			);
			if (ch.room_id) parts.push(`+room_id:${ch.room_id}`);
		} else if (ch.room_id) {
			parts.push(`+room_id:${ch.room_id}`);
		} else {
			parts.push(`+channel_id:${ch.id}`);
		}
	} else if (context.room) {
		parts.push(`+room_id:${context.room.id}`);
	}

	if (negatedChannelFilters.length) {
		parts.push(
			`-channel_id: IN [${negatedChannelFilters.map((f) => f.value).join(" ")}]`,
		);
	}

	return parts;
}

/** Compile before/after date range into a UUID range fragment */
function compileDateRange(before?: string, after?: string): string[] {
	if (!before && !after) return [];

	if (before && after) {
		const from = dateToBoundaryUUID(after, "start");
		const to = dateToBoundaryUUID(before, "end");
		if (from && to) return [`+created_at:[${from} TO ${to}]`];
	} else if (after) {
		const from = dateToBoundaryUUID(after, "start");
		if (from) return [`+created_at:[${from} TO *]`];
	} else if (before) {
		const to = dateToBoundaryUUID(before, "end");
		if (to) return [`+created_at:[* TO ${to}]`];
	}
	return [];
}

/**
 * Convert a date string to a UUID v7 boundary.
 * Start-of-day gives the lowest UUID for that day; end-of-day gives the highest.
 */
function dateToBoundaryUUID(
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
		return UUID.fromFieldsV7(
			unixTsMs,
			0xfff,
			0x3fffffff,
			0xffffffff,
		).toString();
	} catch {
		return undefined;
	}
}
