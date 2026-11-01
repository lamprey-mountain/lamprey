import init, { Parsed, Parser } from "@lamprey/markdown";
import { createResource } from "solid-js";
import type { SerializedDocument } from "./ast";

export const loaded = init();
export const [loadedResource] = createResource(() => loaded);
export { countEmojiOnly } from "./emoji";

export const parser = loaded.then(() => new Parser());
export const [parserResource] = createResource(() => parser);

// cache markdown asts
// TODO: tune this: cache size, lru/lfu, etc?
const AST_CACHE_LIMIT = 1000;
const astCache = new Map<string, SerializedDocument>();

export function parseMarkdown(
	parser: Parser,
	content: string,
): SerializedDocument {
	const cached = astCache.get(content);
	if (cached) {
		// use map as a hacky lru cache
		astCache.delete(content);
		astCache.set(content, cached);
		return cached;
	}

	const ast = parser.parse(content).ast() as SerializedDocument;
	astCache.set(content, ast);

	if (astCache.size > AST_CACHE_LIMIT) {
		const oldest = astCache.keys().next().value;
		if (oldest !== undefined) astCache.delete(oldest);
	}

	return ast;
}

export { Parsed, Parser };
