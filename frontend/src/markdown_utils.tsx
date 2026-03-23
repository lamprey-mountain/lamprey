import { marked, type Token, type Tokens } from "marked";
import { EditorState } from "prosemirror-state";
import {
	Decoration,
	type DecorationAttrs,
	DecorationSet,
} from "prosemirror-view";

type DecorationDef = {
	start: number;
	end: number;
	attrs: DecorationAttrs;
	options?: { inclusiveStart?: boolean; inclusiveEnd?: boolean };
};

let hljs: any = null;
import("highlight.js").then((m) => {
	hljs = m.default;
});

const SYN = { class: "syn" };

const MENTION_CONFIGS = [
	{ type: "user", prefix: "@", regex: /^<@([0-9a-fA-F-]{36})>/ },
	{ type: "role", prefix: "@&", regex: /^<@&([0-9a-fA-F-]{36})>/ },
	{ type: "channel", prefix: "#", regex: /^<#([0-9a-fA-F-]{36})>/ },
	{
		type: "emoji",
		regex: /^<(a?):(\w+):([0-9a-fA-F-]{32,38})>/,
		process: (m: RegExpExecArray) => ({
			animated: !!m[1],
			name: m[2],
			id: m[3],
		}),
	},
];

const mentionExtension = {
	name: "mention",
	level: "inline" as const,
	start: (src: string) => src.indexOf("<"),
	tokenizer(src: string) {
		for (const config of MENTION_CONFIGS) {
			const match = config.regex.exec(src);
			if (match) {
				return {
					type: "mention",
					raw: match[0],
					mention_type: config.type,
					id: match[3] || match[1],
					...(config.process ? config.process(match) : {}),
				};
			}
		}
	},
	renderer(token: any) {
		const attrs = Object.entries(token)
			.filter(([k]) => ["id", "name", "animated"].includes(k))
			.map(([k, v]) => `data-emoji-${k}="${v}"`).join(" ");
		return `<span class="mention" data-mention-type="${token.mention_type}" ${attrs}></span>`;
	},
};

const spoilerExtension = {
	name: "spoiler",
	level: "inline" as const,
	start: (src: string) => src.indexOf("||"),
	tokenizer(src: string) {
		const match = /^\|\|([\s\S]+?)\|\|/.exec(src);
		if (!match) return;
		const token = {
			type: "spoiler",
			raw: match[0],
			text: match[1],
			tokens: [],
		};
		(this as any).lexer.inline(token.text, token.tokens);
		return token;
	},
	renderer(token: any) {
		const content = (this as any).parser.parseInline(token.tokens);
		return `<span class="spoiler" onclick="this.classList.toggle('shown')">${content}</span>`;
	},
};

export const md = marked.use({
	breaks: true,
	gfm: true,
	extensions: [mentionExtension, spoilerExtension],
});

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
	heading: (
		t: Tokens.Heading,
	) => [{ attrs: SYN, start: 0, end: t.depth }],

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
				const hasClosing = lastNewline > firstNewline &&
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
		} else {
			// indented code blocks arent supported
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

function mapDecorations(
	token: Token,
): { len: number; decorations: DecorationDef[] } {
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
	return tokens.reduce((acc, token) => {
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
	}, { pos: startPos, decorations: [] as DecorationDef[] });
}

export function decorate(state: EditorState, placeholderText?: string) {
	const decorations: Decoration[] = [];
	const isEmpty = !state.doc.firstChild?.content.size;

	if (placeholderText && isEmpty) {
		const widget = Decoration.widget(1, () => {
			const span = document.createElement("span");
			span.className = "placeholder";
			span.textContent = placeholderText;
			return span;
		});
		return DecorationSet.create(state.doc, [widget]);
	}

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
