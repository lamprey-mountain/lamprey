import type { Token, Tokens } from "marked";
import {
	type EditorState,
	Plugin,
	PluginKey,
	type Transaction,
} from "prosemirror-state";
import {
	Decoration,
	type DecorationAttrs,
	DecorationSet,
	EditorView,
} from "prosemirror-view";
import { md } from "../../../markdown_utils.tsx";

let hljs: any = null;
import("highlight.js").then((m) => {
	hljs = m.default;
});

const SYN = { class: "syn" };

type DecorationDef = {
	start: number;
	end: number;
	attrs: DecorationAttrs;
	options?: { inclusiveStart?: boolean; inclusiveEnd?: boolean };
};

/**
 * Logic to calculate the syntax delimiter offset (e.g., length of "**")
 */
function getOffset(token: Token): number {
	const offsets: Record<string, number> = {
		strong: 2,
		spoiler: 2,
		em: 1,
		codespan: 1,
	};
	if (token.type === "list_item") {
		return token.raw.match(/^(\s*([-*+]|\d+\.)\s+)/)?.[1].length ?? 0;
	}
	return offsets[token.type] ?? 0;
}

/**
 * Strategy for handling complex syntax highlighting via highlight.js
 */
function getHighlightDecorations(
	content: string,
	lang: string,
	offset: number,
): DecorationDef[] {
	const decos: DecorationDef[] = [];
	if (!hljs) return decos;

	try {
		const highlighted = hljs.highlight(content, {
			language: lang || "plaintext",
		});
		let currentPos = offset;

		const walk = (node: any) => {
			if (typeof node === "string") {
				currentPos += node.length;
			} else if (node.scope) {
				const start = currentPos;
				(node.children || []).forEach(walk);
				decos.push({
					attrs: { class: `hljs-${node.scope.replace(/\./g, " hljs-")}` },
					start,
					end: currentPos,
				});
			} else if (node.children) {
				node.children.forEach(walk);
			}
		};

		highlighted._emitter.root.children.forEach(walk);
	} catch (e) {
		// ignore highlight errors
	}
	return decos;
}

/**
 * Map of token types to decoration generators
 */
const DECORATION_STRATEGIES: Record<string, (token: any) => DecorationDef[]> = {
	heading: (t: Tokens.Heading) => [{ attrs: SYN, start: 0, end: t.depth }],

	em: (t: Tokens.Em) => [
		{ attrs: SYN, start: 0, end: 1 },
		{ attrs: { nodeName: "em" }, start: 1, end: t.raw.length - 1 },
		{ attrs: SYN, start: t.raw.length - 1, end: t.raw.length },
	],

	strong: (t: Tokens.Strong) => [
		{ attrs: SYN, start: 0, end: 2 },
		{ attrs: { nodeName: "b" }, start: 2, end: t.raw.length - 2 },
		{ attrs: SYN, start: t.raw.length - 2, end: t.raw.length },
	],

	spoiler: (t: any) => [
		{ attrs: SYN, start: 0, end: 2 },
		{ attrs: { class: "spoiler-preview" }, start: 2, end: t.raw.length - 2 },
		{ attrs: SYN, start: t.raw.length - 2, end: t.raw.length },
	],

	link: (t: Tokens.Link) => {
		if (t.raw === t.href) {
			return [{ attrs: { class: "link" }, start: 0, end: t.text.length }];
		}
		return [
			{ attrs: SYN, start: 0, end: 1 },
			{ attrs: SYN, start: t.text.length + 1, end: t.text.length + 3 },
			{
				attrs: { class: "link" },
				start: t.text.length + 3,
				end: t.raw.length - 1,
			},
			{ attrs: SYN, start: t.raw.length - 1, end: t.raw.length },
		];
	},

	code: (t: Tokens.Code) => {
		const decos: DecorationDef[] = [];
		const isFenced = t.raw.startsWith("```") || t.raw.startsWith("~~~");

		if (isFenced) {
			const match = t.raw.match(/^([`~]{3,})([a-z-]*)/i);
			const fenceLen = match?.[1].length ?? 3;
			const langLen = match?.[2].length ?? 0;
			const firstNewline = t.raw.indexOf("\n");

			decos.push({ attrs: SYN, start: 0, end: fenceLen });
			decos.push({
				attrs: { class: "syn-code-lang" },
				start: fenceLen,
				end: fenceLen + langLen,
			});

			if (firstNewline !== -1) {
				const lastNewline = t.raw.lastIndexOf("\n");
				const hasClosing =
					lastNewline > firstNewline &&
					t.raw.slice(lastNewline).trim().startsWith(t.raw[0].repeat(3));
				const contentEnd = hasClosing ? lastNewline : t.raw.length;

				decos.push(
					...getHighlightDecorations(
						t.raw.slice(firstNewline + 1, contentEnd),
						t.lang || "",
						firstNewline + 1,
					),
				);

				if (hasClosing) {
					decos.push({
						attrs: SYN,
						start: lastNewline + 1,
						end: t.raw.length,
					});
				}
			}
		}

		return decos;
	},

	blockquote: (t: Tokens.Blockquote) => {
		const decos: DecorationDef[] = [];
		let pos = 0;
		for (const line of t.raw.split("\n")) {
			const m = line.match(/^(\s*>+)/);
			if (m) {
				decos.push({ attrs: SYN, start: pos, end: pos + m[1].length });
			}
			pos += line.length + 1;
		}
		return decos;
	},

	list_item: (t: Tokens.ListItem) => {
		const m = t.raw.match(/^(\s*([-*+]|\d+\.)\s+)/);
		return m ? [{ attrs: SYN, start: 0, end: m[1].length }] : [];
	},
};

function mapDecorations(token: Token): {
	len: number;
	decorations: DecorationDef[];
} {
	const decorations = DECORATION_STRATEGIES[token.type]?.(token) ?? [];

	if ("tokens" in token && token.tokens && token.type !== "blockquote") {
		decorations.push(
			...reduceDecorations(token.tokens, getOffset(token)).decorations,
		);
	}
	if ("items" in token && (token as any).items) {
		decorations.push(...reduceDecorations((token as any).items, 0).decorations);
	}

	return { decorations, len: token.raw.length };
}

function reduceDecorations(tokens: Token[], startPos = 0) {
	return tokens.reduce(
		(acc, token) => {
			const { decorations, len } = mapDecorations(token);
			const mapped = decorations.map((d) => ({
				...d,
				start: d.start + acc.pos,
				end: d.end + acc.pos,
			}));
			return {
				pos: acc.pos + len,
				decorations: [...acc.decorations, ...mapped],
			};
		},
		{ pos: startPos, decorations: [] as DecorationDef[] },
	);
}

function calculateDecorations(state: EditorState): DecorationSet {
	const decorations: Decoration[] = [];

	state.doc.descendants((node, pos) => {
		if (node.isText && node.text) {
			const tokens = md.lexer(node.text);
			const { decorations: decoDefs } = reduceDecorations(tokens, pos);
			for (const d of decoDefs) {
				decorations.push(Decoration.inline(d.start, d.end, d.attrs, d.options));
			}
			return false;
		}
	});

	return DecorationSet.create(state.doc, decorations);
}

/**
 * Plugin state interface
 */
interface MarkdownHighlightState {
	decorations: DecorationSet;
}

const markdownHighlightKey = new PluginKey<MarkdownHighlightState>(
	"markdown-highlight",
);

/**
 * Create a plugin that manages markdown syntax highlighting decorations.
 * Decorations are only recalculated for nodes that changed.
 */
export function createMarkdownHighlightPlugin() {
	return new Plugin<MarkdownHighlightState>({
		key: markdownHighlightKey,
		state: {
			init(_, state) {
				return {
					decorations: calculateDecorations(state),
				};
			},
			apply(tr, prev, _oldState, newState) {
				// Map old decorations through the transaction
				const mapped = prev.decorations.map(tr.mapping, tr.doc);

				// Only recalculate if document content changed
				if (tr.docChanged) {
					// Calculate decorations for the new document
					const newDecorations = calculateDecorations(newState);
					return { decorations: newDecorations };
				}

				return { decorations: mapped };
			},
		},
		props: {
			decorations(state) {
				return this.getState(state)?.decorations ?? DecorationSet.empty;
			},
		},
	});
}

/**
 * Get the current decoration set from the plugin state
 */
export function getMarkdownDecorations(state: EditorState): DecorationSet {
	const pluginState = markdownHighlightKey.getState(state);
	return pluginState?.decorations ?? DecorationSet.empty;
}
