import { diffWords } from "diff";
import { DOMParser, type Node as PMNode } from "prosemirror-model";
import type { LampreyComponent, Serdoc } from "ts-sdk";
import type { DiffMark } from "../features/editor/diff-plugin";
import { schema } from "../features/editor/schema";

export function serdocToDoc(serdoc: Serdoc) {
	const htmlParts: string[] = [];
	for (const c of serdoc.components) {
		if (c.type === "Text") {
			htmlParts.push(c.content);
		}
	}
	if (htmlParts.length === 0) {
		htmlParts.push("<p></p>");
	}
	// FIXME: potential xss
	const div = document.createElement("div");
	div.innerHTML = htmlParts.join("");
	return DOMParser.fromSchema(schema).parse(div);
}

// TODO: alternative fn that doesn't require DOMParser?
export function serdocToDoc2(serdoc: Serdoc): PMNode {
	const nodes = serdoc.components.flatMap(componentToNodes);
	return schema.node("doc", null, nodes);
}

function componentToNodes(comp: LampreyComponent): PMNode[] {
	switch (comp.type) {
		case "Text":
			return [
				schema.node("paragraph", null, [schema.text(comp.content || "")]),
			];
		case "Container":
		case "Section":
			return comp.components.flatMap(componentToNodes);
		case "Details":
			return [
				...comp.summary.flatMap(componentToNodes),
				...comp.details.flatMap(componentToNodes),
			];
		default:
			return [];
	}
}

export function computeDiffMarks(
	oldSerdoc: Serdoc,
	newSerdoc: Serdoc,
): DiffMark[] {
	const oldDoc = serdocToDoc(oldSerdoc);
	const newDoc = serdocToDoc(newSerdoc);

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

// NOTE: maybe see frontend/src/components/features/editor/markdown-highlight-plugin.ts
// buildTextAndSegments() and toDocPos() may be relevant and/or useful here
