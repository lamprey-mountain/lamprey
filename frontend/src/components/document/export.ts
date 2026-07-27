import type { EditorView } from "prosemirror-view";
import { parser as markdownParser } from "@/lib/markdown";
import { serializeToMarkdown } from "../features/editor/serializer";
import htmlTemplate from "./html-template.html?raw";

export function exportAsMarkdown(view: EditorView, filename: string) {
	const markdown = serializeToMarkdown(view.state.doc);
	downloadFile(markdown, filename, "text/markdown");
}

export async function exportAsHtml(
	view: EditorView,
	filename: string,
	title: string,
) {
	const markdown = serializeToMarkdown(view.state.doc);
	const md = await markdownParser;
	const htmlContent = md.parse(markdown).toHTML();
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
