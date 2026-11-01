import type { Parser as MarkdownParser, MentionData } from "@lamprey/markdown";
import { diffWords } from "diff";
import { DOMParser, type Node as PMNode } from "prosemirror-model";
import type { LampreyComponent, Serdoc } from "ts-sdk";
import type { DiffMark } from "../features/editor/diff-plugin";
import { schema } from "../features/editor/schema";

export type MentionNameResolver = (
	mention: MentionData,
) => string | null | undefined;

type Span = { start: number; end: number };
type ParsedMention = MentionData & { text: string; span: Span };
type ParsedEmoji = (
	| { kind: "Custom"; id: string; name: string; animated: boolean }
	| { kind: "Unicode" }
) & { text: string; span: Span };

export function serdocToDoc(
	serdoc: Serdoc,
	parser: MarkdownParser,
	resolveMentionName?: MentionNameResolver,
): PMNode {
	const htmlParts: string[] = [];
	for (const component of serdoc.components) {
		collectComponentHTML(component, htmlParts, parser, resolveMentionName);
	}
	if (htmlParts.length === 0) htmlParts.push("<p><br></p>");

	const div = document.createElement("div");
	div.innerHTML = htmlParts.join("");
	return DOMParser.fromSchema(schema).parse(div);
}

function collectComponentHTML(
	component: LampreyComponent,
	htmlParts: string[],
	parser: MarkdownParser,
	resolveMentionName?: MentionNameResolver,
) {
	switch (component.type) {
		case "Text":
			htmlParts.push(
				textComponentToHTML(component.content, parser, resolveMentionName),
			);
			break;
		case "Container":
		case "Section":
			for (const child of component.components) {
				collectComponentHTML(child, htmlParts, parser, resolveMentionName);
			}
			break;
		case "Details":
			for (const child of component.summary) {
				collectComponentHTML(child, htmlParts, parser, resolveMentionName);
			}
			for (const child of component.details) {
				collectComponentHTML(child, htmlParts, parser, resolveMentionName);
			}
			break;
	}
}

function textComponentToHTML(
	content: string,
	parser: MarkdownParser,
	resolveMentionName?: MentionNameResolver,
): string {
	const parsed = parser.parse(content);
	const replacements: { start: number; end: number; html: string }[] = [];

	// the upstream d.ts does not match the wasm runtime shape
	for (const raw of parsed.mentions() as unknown[]) {
		const mention = normalizeMention(raw);
		if (!mention) continue;
		replacements.push({
			...mention.span,
			html: mentionToHTML(mention, resolveMentionName),
		});
	}
	for (const raw of parsed.emoji() as unknown[]) {
		const emoji = normalizeEmoji(raw);
		if (!emoji) continue;
		replacements.push({ ...emoji.span, html: emojiToHTML(emoji) });
	}

	replacements.sort((a, b) => a.start - b.start || a.end - b.end);

	let html = "";
	let pos = 0;
	for (const replacement of replacements) {
		if (replacement.start < pos) continue;
		html += escapeHTML(content.slice(pos, replacement.start));
		html += replacement.html;
		pos = replacement.end;
	}
	html += escapeHTML(content.slice(pos));

	return `<p>${html}</p>`;
}

function getField<T>(value: unknown, key: string): T | undefined {
	if (value instanceof Map) return value.get(key) as T | undefined;
	if (typeof value !== "object" || value === null) return undefined;
	return (value as Record<string, T>)[key];
}

function normalizeSpan(value: unknown): Span | null {
	const start = getField<number>(value, "start");
	const end = getField<number>(value, "end");
	if (typeof start !== "number" || typeof end !== "number") return null;
	return { start, end };
}

function normalizeMention(value: unknown): ParsedMention | null {
	const text = getField<string>(value, "text");
	const span = normalizeSpan(getField<unknown>(value, "span"));
	const type = getField<string>(value, "type");
	if (typeof text !== "string" || !span) return null;

	if (type === "Everyone") return { type, text, span };
	if (type !== "User" && type !== "Role" && type !== "Channel") return null;

	const id = getField<string>(value, "id");
	if (typeof id !== "string") return null;
	return { type, id, text, span };
}

function normalizeEmoji(value: unknown): ParsedEmoji | null {
	const text = getField<string>(value, "text");
	const span = normalizeSpan(getField<unknown>(value, "span"));
	const kind = getField<string>(value, "kind");
	if (typeof text !== "string" || !span) return null;
	if (kind !== "Custom") return { kind: "Unicode", text, span };

	const id = getField<string>(value, "id");
	const name = getField<string>(value, "name");
	const animated = getField<boolean>(value, "animated");
	if (typeof id !== "string" || typeof name !== "string") return null;

	return { kind: "Custom", id, name, animated: !!animated, text, span };
}

function mentionToHTML(
	mention: ParsedMention,
	resolveMentionName?: MentionNameResolver,
): string {
	const name = resolveMentionName?.(mention);
	const nameAttr =
		name == null || name === "" ? "" : ` data-name="${escapeHTMLAttr(name)}"`;
	const text = escapeHTML(mention.text);

	switch (mention.type) {
		case "User":
			return `<span data-user-id="${escapeHTMLAttr(
				mention.id,
			)}"${nameAttr} class="mention">${text}</span>`;
		case "Role":
			return `<span data-role-id="${escapeHTMLAttr(
				mention.id,
			)}"${nameAttr} class="mention">${text}</span>`;
		case "Channel":
			return `<span data-channel-id="${escapeHTMLAttr(
				mention.id,
			)}"${nameAttr} class="mention">${text}</span>`;
		case "Everyone":
			return `<span data-mention="everyone" class="mention mention-everyone">${text}</span>`;
	}

	return "";
}

function emojiToHTML(emoji: ParsedEmoji): string {
	if (emoji.kind === "Custom") {
		return `<span data-emoji-id="${escapeHTMLAttr(
			emoji.id,
		)}" data-emoji-name="${escapeHTMLAttr(emoji.name)}" data-emoji-animated="${
			emoji.animated ? "true" : "false"
		}" class="mention">${escapeHTML(emoji.text)}</span>`;
	}

	return `<span data-emoji-unicode="${escapeHTMLAttr(
		emoji.text,
	)}" class="emoji-unicode">${escapeHTML(emoji.text)}</span>`;
}

function escapeHTML(value: string): string {
	return value
		.replace(/&/g, "&amp;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;");
}

function escapeHTMLAttr(value: string): string {
	return escapeHTML(value).replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

export function computeDiffMarks(oldDoc: PMNode, newDoc: PMNode): DiffMark[] {
	const oldData = getDocTextAndMap(oldDoc);
	const newData = getDocTextAndMap(newDoc);

	const changes = diffWords(oldData.text, newData.text);

	const marks: DiffMark[] = [];
	let oldTextPos = 0;
	let newTextPos = 0;

	for (const change of changes) {
		const len = change.value.length;

		if (change.added) {
			const from = mapTextPosToPMPos(newData.posMap, newTextPos, newDoc);
			const to = mapTextPosToPMPos(newData.posMap, newTextPos + len, newDoc);
			if (from < to) {
				marks.push({ type: "insertion", from, to });
			}
			newTextPos += len;
		} else if (change.removed) {
			const pos = mapTextPosToPMPos(oldData.posMap, oldTextPos, oldDoc);
			const cleanText = change.value.replace(/\n/g, " ↵ ");
			marks.push({ type: "deletion", pos, text: cleanText });
			oldTextPos += len;
		} else {
			oldTextPos += len;
			newTextPos += len;
		}
	}

	return marks;
}

function getDocTextAndMap(doc: PMNode): { text: string; posMap: number[] } {
	let text = "";
	const posMap: number[] = [];

	doc.descendants((node, pos) => {
		if (node.isText) {
			const str = node.text!;
			for (let i = 0; i < str.length; i++) {
				posMap.push(pos + i);
			}
			text += str;
		} else if (node.isBlock) {
			if (text.length > 0 && text[text.length - 1] !== "\n") {
				text += "\n";
				posMap.push(pos);
			}
		}
		return true;
	});

	return { text, posMap };
}

function mapTextPosToPMPos(
	posMap: number[],
	textPos: number,
	doc: PMNode,
): number {
	const maxPos = Math.max(1, doc.content.size);
	if (textPos < 0) return 0;
	if (textPos >= posMap.length) return maxPos;
	return posMap[textPos] ?? maxPos;
}
