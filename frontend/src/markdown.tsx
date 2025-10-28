import { marked, type Token } from "marked";
import { EditorState } from "prosemirror-state";
import { Decoration, DecorationAttrs, DecorationSet } from "prosemirror-view";

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

export const md = marked.use({
	breaks: true,
	gfm: true,
	extensions: [mentionExtension],
	renderer: {
		del({ tokens }) {
			return `<s>${this.parser.parseInline(tokens)}</s>`;
		},
	},
});

// TODO: refactor
export function decorate(state: EditorState, placeholderText?: string) {
	if (state.doc.firstChild!.firstChild === null) {
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
				// does this work with indented code blocks?
				const firstEnd = ast.raw.indexOf("\n");
				return [
					{ attrs: { class: "syn" }, start: 0, end: firstEnd },
					// { attrs: { nodeName: "pre" }, start: firstEnd + 1, end: ast.text.length + firstEnd + 1 },
					// { attrs: { class: "font-mono" }, start: firstEnd + 1, end: ast.text.length + firstEnd + 1 },
					{
						attrs: { nodeName: "code" },
						start: firstEnd + 1,
						end: ast.text.length + firstEnd + 1,
					},
					{
						attrs: { class: "syn" },
						start: ast.text.length + firstEnd + 2,
						end: ast.raw.length,
					},
				];
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
			case "mention": {
				return [{
					attrs: { class: "mention" },
					start: 0,
					end: ast.raw.length,
				}];
			}
			// case "blockquote": {
			// 	// // FIXME: breaks on multiline blockquotes "> foo\n> bar"
			// 	// const synLen = ast.raw.length - ast.text.length;
			// 	// decorations.push(Decoration.inline(pos, pos + synLen, { class: "syn" }));
			// 	// pos += synLen;
			// 	// ast.tokens?.forEach(walk);

			// 	// FIXME: format recursively using ast.tokens trickery or a better library
			// 	// console.log({ ast })
			// 	for (const line of ast.raw.split("\n")) {
			// 		// console.log({ pos, line })
			// 		if (line.startsWith(">")) {
			// 			decorations.push(
			// 				Decoration.inline(pos + 1, pos + 2, { class: "syn" }),
			// 			);
			// 		}
			// 		pos += line.length + 1;
			// 		// ast.tokens?.forEach(walk);
			// 	}
			// 	return;
			// }
			// case "list": {
			// 	ast.items.forEach(walk);
			// 	return;
			// }
			// case "list_item": {
			// 	const endLen = ast.raw.match(/\n+$/)?.[0].length ?? 0;
			// 	const startLen = ast.raw.length - ast.text.length - endLen;
			// 	decorations.push(
			// 		Decoration.inline(pos, pos + startLen, { class: "syn" }),
			// 	);
			// 	pos += startLen;
			// 	ast.tokens?.forEach(walk);
			// 	pos += endLen;
			// 	return;
			// }
			default: {
				return [];
			}
		}
	}

	function getOffset(ty: string) {
		switch (ty) {
			case "strong":
				return 2;
			case "em":
				return 1;
			case "codespan":
				return 1;
			default:
				return 0;
		}
	}

	type A = { start: number; end: number; attrs: DecorationAttrs };

	function mapDecorations(
		ast: Token,
	): { len: number; decorations: Array<A> } {
		const decorations = [];
		decorations.push(...extraDecorations(ast));
		if ("tokens" in ast) {
			decorations.push(
				...reduceDecorations(ast.tokens!, getOffset(ast.type))
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
		reduced.decorations.map((i) => Decoration.inline(i.start, i.end, i.attrs)),
	);
}
