import {
	type DecorationKind,
	type Parsed as Markdown,
	Parser as MarkdownParser,
} from "@lamprey/markdown";
import type Hljs from "highlight.js";
import { type EditorState, Plugin, PluginKey } from "prosemirror-state";
import {
	Decoration,
	type DecorationAttrs,
	DecorationSet,
} from "prosemirror-view";
import { loaded as mdLoaded } from "@/lib/markdown";

let markdown: Markdown;

let hljs: typeof Hljs | null = null;
import("highlight.js").then((m) => {
	hljs = m.default;
});

// TODO: wire this into calculateDecorations
function getHighlightDecorations(
	content: string,
	lang: string,
	baseOffset: number, // text offset where `content` starts
): Decoration[] {
	if (!hljs) return [];

	const decos: Decoration[] = [];
	try {
		const highlighted = hljs.highlight(content, {
			language: lang || "plaintext",
		});

		let pos = baseOffset;
		const walk = (node: unknown) => {
			if (typeof node === "string") {
				pos += node.length;
				return;
			}

			const n = node as { scope?: string; children?: unknown[] };
			if (n.scope) {
				const start = pos;
				(n.children || []).forEach(walk);
				decos.push(
					Decoration.inline(start, pos, {
						class: `hljs-${n.scope.replace(/\./g, " hljs-")}`,
					}),
				);
			} else {
				(n.children || []).forEach(walk);
			}
		};

		// highlighted structure varies by hljs version
		// HACK: use something else instead of highlight.js that supports manual highlighting?
		const root =
			(highlighted as any)._emitter?.root || (highlighted as any).value;
		if (root?.children) {
			root.children.forEach(walk);
		}
	} catch (_e) {
		// ignore highlight errors
	}

	return decos;
}

function getAttrs(kind: DecorationKind): DecorationAttrs {
	switch (kind) {
		case "Syntax":
			return { class: "syn" };
		case "Emphasis":
			return { nodeName: "em" };
		case "Strong":
			return { nodeName: "b" };
		case "Code":
			return { nodeName: "code" };
		case "Spoiler":
			return { class: "spoiler-preview" };
		case "Strikethrough":
			return { nodeName: "s" };
		case "Link":
			return { class: "link" };
		default:
			return {};
	}
}

type Segment = { docPos: number; textPos: number; length: number };

/** get serialized text from the editor, along with a list of offsets */
function buildTextAndSegments(state: EditorState) {
	let text = "";
	const segments: Segment[] = [];
	state.doc.descendants((node, pos) => {
		if (node.isText && node.text) {
			segments.push({
				docPos: pos,
				textPos: text.length,
				length: node.text.length,
			});
			text += node.text;
			return false;
		}
		if (node.isBlock && !node.isTextblock) return true; // skip non-leaf containers
		if (node.isBlock) {
			segments.push({ docPos: pos, textPos: text.length, length: 1 });
			text += "\n"; // block boundary
		}
		return true;
	});
	return { text, segments };
}

/** find the mapped document position for an offset */
function toDocPos(offset: number, segments: Segment[]): number {
	// binary search: find segment containing offset
	let lo = 0,
		hi = segments.length - 1;
	while (lo < hi) {
		const mid = (lo + hi + 1) >> 1;
		if (segments[mid].textPos <= offset) lo = mid;
		else hi = mid - 1;
	}
	const seg = segments[lo];
	return seg.docPos + Math.min(offset - seg.textPos, seg.length);
}

function calculateDecorations(state: EditorState): DecorationSet {
	if (!markdown) return DecorationSet.empty;

	const { text, segments } = buildTextAndSegments(state);
	markdown.edit(0, markdown.sourceLength, text);

	const decos = markdown
		.decorations()
		.map((d) =>
			Decoration.inline(
				toDocPos(d.span.start, segments),
				toDocPos(d.span.end, segments),
				getAttrs(d.kind),
			),
		);

	// TODO: find codeblocks, use hljs to calculate decorations

	return DecorationSet.create(state.doc, decos);
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
				if (tr.docChanged || tr.getMeta(markdownHighlightKey) === "reload") {
					return { decorations: calculateDecorations(newState) };
				}

				return { decorations: prev.decorations.map(tr.mapping, tr.doc) };
			},
		},
		view(view) {
			mdLoaded.then(() => {
				if (view.isDestroyed) return;
				const parser = new MarkdownParser();
				markdown = parser.empty();

				view.dispatch(view.state.tr.setMeta(markdownHighlightKey, "reload"));
			});

			return {};
		},
		props: {
			decorations(state) {
				return this.getState(state)?.decorations ?? DecorationSet.empty;
			},
		},
	});
}
