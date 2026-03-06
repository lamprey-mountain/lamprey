import { marked, type Token } from "marked";
import { EditorState } from "prosemirror-state";
import { Decoration, DecorationAttrs, DecorationSet } from "prosemirror-view";

let hljs: any = null;
import("highlight.js").then((m) => {
	hljs = m.default;
});

const mentionExtension = {
	name: "mention",
	level: "inline" as const,
	start(src: string) {
		return src.indexOf("<");
	},
	tokenizer(src: string) {
		const userMention =
			/^<@([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/
				.exec(src);
		if (userMention) {
			return {
				type: "mention",
				raw: userMention[0],
				mention_type: "user",
				id: userMention[1],
			};
		}
		const roleMention =
			/^<@&([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/
				.exec(src);
		if (roleMention) {
			return {
				type: "mention",
				raw: roleMention[0],
				mention_type: "role",
				id: roleMention[1],
			};
		}
		const channelMention =
			/^<#([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/
				.exec(src);
		if (channelMention) {
			return {
				type: "mention",
				raw: channelMention[0],
				mention_type: "channel",
				id: channelMention[1],
			};
		}
		const emojiMention =
			/^<(a?):(\w+):([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{12})>/
				.exec(src);
		if (emojiMention) {
			return {
				type: "mention",
				raw: emojiMention[0],
				mention_type: "emoji",
				animated: !!emojiMention[1],
				name: emojiMention[2],
				id: emojiMention[3],
			};
		}
		return undefined;
	},
	renderer(token: any) {
		if (token.mention_type === "user") {
			return `<span class="mention" data-mention-type="user" data-user-id="${token.id}"></span>`;
		}
		if (token.mention_type === "role") {
			return `<span class="mention" data-mention-type="role" data-role-id="${token.id}"></span>`;
		}
		if (token.mention_type === "channel") {
			return `<span class="mention" data-mention-type="channel" data-channel-id="${token.id}"></span>`;
		}
		if (token.mention_type === "emoji") {
			return `<span class="mention" data-mention-type="emoji" data-emoji-id="${token.id}" data-emoji-name="${token.name}" data-emoji-animated="${token.animated}"></span>`;
		}
		return token.raw;
	},
};

const spoilerExtension = {
	name: "spoiler",
	level: "inline" as const,
	start(src: string) {
		return src.indexOf("||");
	},
	tokenizer(src: string) {
		const rule = /^\|\|([\s\S]+?)\|\|/;
		const match = rule.exec(src);
		if (match) {
			const token = {
				type: "spoiler",
				raw: match[0],
				text: match[1],
				tokens: [] as any[],
			};
			(this as any).lexer.inline(token.text, token.tokens);
			return token;
		}
	},
	renderer(token: any) {
		return `<span class="spoiler" onclick="this.classList.toggle('shown')">${
			(this as any).parser.parseInline(token.tokens)
		}</span>`;
	},
};

export const md = marked.use({
	breaks: true,
	gfm: true,
	extensions: [mentionExtension, spoilerExtension],
	renderer: {
		del({ tokens }) {
			return `<s>${this.parser.parseInline(tokens)}</s>`;
		},
	},
});

export function decorate(state: EditorState, placeholderText?: string) {
	if (state.doc.firstChild!.firstChild === null) {
		// FIXME: reactive placeholder text
		const placeholder = (
			<div class="placeholder" role="presentation">
				{/* @once */ placeholderText}
			</div>
		) as HTMLDivElement;
		return DecorationSet.create(state.doc, [
			Decoration.widget(0, placeholder),
		]);
	}

	function extraDecorations(ast: Token) {
		switch (ast.type) {
			case "heading": {
				return [{ attrs: { class: "syn" }, start: 0, end: ast.depth }];
			}
			case "spoiler" as any: {
				return [
					{ attrs: { class: "syn" }, start: 0, end: 2 },
					{
						attrs: { class: "spoiler-preview" },
						start: 2,
						end: ast.raw.length - 2,
					},
					{
						attrs: { class: "syn" },
						start: ast.raw.length - 2,
						end: ast.raw.length,
					},
				];
			}
			case "em": {
				return [
					{ attrs: { class: "syn" }, start: 0, end: 1 },
					{
						attrs: { nodeName: "em" },
						start: 1,
						end: ast.raw.length - 1,
					},
					{
						attrs: { class: "syn" },
						start: ast.raw.length - 1,
						end: ast.raw.length,
					},
				];
			}
			case "link": {
				if (ast.raw === ast.href) {
					return [{
						attrs: { style: "color: var(--color-link)" },
						start: 0,
						end: ast.text.length,
					}];
				} else {
					return [
						{ attrs: { class: "syn" }, start: 0, end: 1 },
						{
							attrs: { class: "syn" },
							start: ast.text.length + 1,
							end: ast.text.length + 3,
						},
						{
							attrs: { style: "color: var(--color-link)" },
							start: ast.text.length + 3,
							end: ast.raw.length - 1,
						},
						{
							attrs: { class: "syn" },
							start: ast.raw.length - 1,
							end: ast.raw.length,
						},
					];
				}
			}
			case "strong": {
				return [
					{ attrs: { class: "syn" }, start: 0, end: 2 },
					{ attrs: { nodeName: "b" }, start: 2, end: ast.raw.length - 2 },
					{
						attrs: { class: "syn" },
						start: ast.raw.length - 2,
						end: ast.raw.length,
					},
				];
			}
			case "code": {
				const decorations = [];
				const isFenced = ast.raw.startsWith("```") || ast.raw.startsWith("~~~");
				const lang = (ast as any).lang;

				function getHighlightDecorations(
					content: string,
					language: string,
					offset: number,
				) {
					const decos: any[] = [];
					if (!hljs) return decos;
					try {
						const highlighted = hljs.highlight(content, {
							language: language || "plaintext",
						});
						// hljs 11+ internal token tree traversal
						let currentPos = offset;
						const walk = (node: any) => {
							if (typeof node === "string") {
								currentPos += node.length;
							} else if (node.scope) {
								const start = currentPos;
								for (const subNode of node.children) {
									walk(subNode);
								}
								decos.push({
									attrs: {
										class: `hljs-${node.scope.replace(/\./g, " hljs-")}`,
									},
									start,
									end: currentPos,
								});
							} else if (node.children) {
								for (const subNode of node.children) {
									walk(subNode);
								}
							}
						};

						// Accessing internal _emitter.root children for hljs 11+
						const root = (highlighted as any)._emitter.root;
						for (const node of root.children) {
							walk(node);
						}
					} catch (e) {
						// Fallback or ignore if language unknown
					}
					return decos;
				}

				if (isFenced) {
					const firstEnd = ast.raw.indexOf("\n");
					if (firstEnd === -1) {
						decorations.push({
							attrs: { class: "syn" },
							start: 0,
							end: ast.raw.length,
						});
					} else {
						// Opening fence
						decorations.push({
							attrs: { class: "syn" },
							start: 0,
							end: firstEnd,
						});

						let lastNewline = ast.raw.lastIndexOf("\n");
						if (lastNewline === ast.raw.length - 1) {
							lastNewline = ast.raw.lastIndexOf("\n", lastNewline - 1);
						}

						if (lastNewline > firstEnd) {
							const lastLine = ast.raw.slice(lastNewline + 1);
							const fenceChar = ast.raw[0];
							// Check if last line is actually a closing fence
							const lastLineTrimmed = lastLine.trim();
							const hasClosingFence =
								lastLineTrimmed.startsWith(fenceChar.repeat(3)) &&
								lastLineTrimmed.slice(3).replace(new RegExp(fenceChar, "g"), "")
										.trim() === "";

							if (hasClosingFence) {
								const content = ast.raw.slice(firstEnd + 1, lastNewline);
								decorations.push({
									attrs: { class: "code-block font-mono" },
									start: firstEnd + 1,
									end: lastNewline,
								});
								decorations.push(
									...getHighlightDecorations(content, lang, firstEnd + 1),
								);
								decorations.push({
									attrs: { class: "syn" },
									start: lastNewline + 1,
									end: ast.raw.length,
								});
							} else {
								const content = ast.raw.slice(firstEnd + 1);
								decorations.push({
									attrs: { class: "code-block font-mono" },
									start: firstEnd + 1,
									end: ast.raw.length,
									options: { inclusiveEnd: true },
								});
								decorations.push(
									...getHighlightDecorations(content, lang, firstEnd + 1),
								);
							}
						} else {
							const content = ast.raw.slice(firstEnd + 1);
							decorations.push({
								attrs: { class: "code-block font-mono" },
								start: firstEnd + 1,
								end: ast.raw.length,
								options: { inclusiveEnd: true },
							});
							decorations.push(
								...getHighlightDecorations(content, lang, firstEnd + 1),
							);
						}
					}
				} else {
					// Indented code block
					decorations.push({
						attrs: { class: "code-block font-mono" },
						start: 0,
						end: ast.raw.length,
						options: { inclusiveEnd: true },
					});
					decorations.push(
						...getHighlightDecorations(ast.raw, lang, 0),
					);
				}

				return decorations;
			}
			case "codespan": {
				return [
					{ attrs: { class: "syn" }, start: 0, end: 1 },
					{
						attrs: { nodeName: "code" },
						start: 1,
						end: ast.raw.length - 1,
					},
					{
						attrs: { class: "syn" },
						start: ast.raw.length - 1,
						end: ast.raw.length,
					},
				];
			}
			case "blockquote": {
				const decorations = [];
				const lines = ast.raw.split("\n");
				let currentPos = 0;
				for (const line of lines) {
					const match = line.match(/^(\s*>+)/);
					if (match) {
						decorations.push({
							attrs: { class: "syn" },
							start: currentPos,
							end: currentPos + match[1].length,
						});
					}
					currentPos += line.length + 1;
				}
				return decorations;
			}
			case "list_item": {
				const match = ast.raw.match(/^(\s*([-*+]|\d+\.)\s+)/);
				if (match) {
					return [{
						attrs: { class: "syn" },
						start: 0,
						end: match[1].length,
					}];
				}
				return [];
			}
			default: {
				return [];
			}
		}
	}

	function getOffset(ast: Token) {
		switch (ast.type) {
			case "strong":
				return 2;
			case "spoiler" as any:
				return 2;
			case "em":
				return 1;
			case "codespan":
				return 1;
			case "list_item": {
				const match = ast.raw.match(/^(\s*([-*+]|\d+\.)\s+)/);
				return match ? match[1].length : 0;
			}
			default:
				return 0;
		}
	}

	type A = {
		start: number;
		end: number;
		attrs: DecorationAttrs;
		options?: { inclusiveStart?: boolean; inclusiveEnd?: boolean };
	};

	function mapDecorations(
		ast: Token,
	): { len: number; decorations: Array<A> } {
		const decorations = [];
		decorations.push(...extraDecorations(ast));
		if ("tokens" in ast && ast.type !== "blockquote") {
			decorations.push(
				...reduceDecorations(ast.tokens!, getOffset(ast))
					.decorations,
			);
		}
		if ("items" in ast) {
			decorations.push(
				...reduceDecorations((ast as any).items, 0)
					.decorations,
			);
		}
		return {
			decorations,
			len: ast.raw.length,
		};
	}

	function reduceDecorations(tokens: Array<Token>, startPos = 0) {
		return tokens.map(mapDecorations)
			.reduce(({ pos, decorations }, i) => ({
				pos: pos + i.len,
				decorations: [
					...decorations,
					...i.decorations.map((j: A) => ({
						start: j.start + pos,
						end: j.end + pos,
						attrs: j.attrs,
						options: j.options,
					})),
				],
			}), { pos: startPos, decorations: [] as Array<A> });
	}

	/*
	some nice colors from an old project
		--background-1: #24262b;
--background-2: #1e2024;
--background-3: #191b1d;
--background-4: #17181a;
--foreground-1: #eae8efcc;
--foreground-2: #eae8ef9f;
--foreground-link: #b18cf3;
--foreground-danger: #fa6261;
	*/

	const reduced = reduceDecorations(md.lexer(state.doc.textContent), 1);
	return DecorationSet.create(
		state.doc,
		reduced.decorations.map((i) =>
			Decoration.inline(i.start, i.end, i.attrs, i.options)
		),
	);
}
