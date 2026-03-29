import {
	type Fragment,
	Node as ProsemirrorNode,
	Slice,
} from "prosemirror-model";
import type { EditorView } from "prosemirror-view";
import { md } from "@/markdown_utils";
import htmlTemplate from "./html-template.html?raw";

/**
 * Converts a ProseMirror Document, Fragment, or Slice into a Markdown string.
 */
export function serializeToMarkdown(
	input: ProsemirrorNode | Fragment | Slice,
): string {
	// 1. Extract the fragment (the actual list of nodes)
	let fragment: Fragment;
	if (input instanceof Slice) {
		fragment = input.content;
	} else if (input instanceof ProsemirrorNode) {
		fragment = input.content;
	} else {
		fragment = input;
	}

	let markdown = "";

	fragment.forEach((node) => {
		switch (node.type.name) {
			case "blockquote": {
				// every line should start with a "> "
				node.forEach((child) => {
					const lines = child.textContent.split("\n");
					lines.forEach((line) => {
						const cleanLine = line.trimStart().startsWith(">")
							? line.trimStart().slice(1).trimStart()
							: line;
						markdown += `> ${cleanLine}\n`;
					});
				});
				markdown += "\n";
				break;
			}

			default: {
				if (node.isInline) {
					markdown += node.textContent;
				} else {
					markdown += node.textContent + "\n\n";
				}
			}
		}
	});

	return markdown.trim();
}

export function exportAsMarkdown(view: EditorView, filename: string) {
	const markdown = serializeToMarkdown(view.state.doc);
	downloadFile(markdown, filename, "text/markdown");
}

export function exportAsHtml(
	view: EditorView,
	filename: string,
	title: string,
) {
	const markdown = serializeToMarkdown(view.state.doc);
	const tokens = md.lexer(markdown);
	const htmlContent = md.parser(tokens);
	const fullHtml = generateHtmlDocument(title, htmlContent);
	downloadFile(fullHtml, filename, "text/html");
}

/**
 * Triggers a browser download for the given content.
 */
export function downloadFile(
	content: string,
	filename: string,
	mimeType: string = "text/plain",
) {
	const blob = new Blob([content], { type: mimeType });
	const url = URL.createObjectURL(blob);
	const a = document.createElement("a");
	a.href = url;
	a.download = filename;
	document.body.appendChild(a);
	a.click();
	document.body.removeChild(a);
	URL.revokeObjectURL(url);
}

/**
 * Escapes HTML special characters.
 */
function escapeHtml(text: string): string {
	return text
		.replace(/&/g, "&amp;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;")
		.replace(/"/g, "&quot;")
		.replace(/'/g, "&#039;");
}

/**
 * Generates a complete HTML document with embedded styles.
 */
function generateHtmlDocument(title: string, content: string): string {
	return htmlTemplate
		.replace("TITLE", escapeHtml(title))
		.replace("CONTENT", content);
}

/**
 * Generates a filename for export based on channel/document info.
 */
export function generateFilename(
	channelName: string = "document",
	extension: string = "md",
): string {
	// Sanitize the name for use as a filename
	const sanitized = channelName
		.toLowerCase()
		.replace(/[^a-z0-9-_]+/g, "-")
		.replace(/^-+|-+$/g, "")
		.slice(0, 50);

	return `${sanitized || "document"}.${extension}`;
}
