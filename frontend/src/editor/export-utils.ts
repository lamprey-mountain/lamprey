import type { EditorView } from "prosemirror-view";
import { md } from "../markdown_utils.tsx";
import htmlTemplate from "./html-template.html?raw";

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
 * Exports the current editor content as Markdown.
 * Gets the raw text content from the ProseMirror document.
 */
export function exportAsMarkdown(
	view: EditorView,
	filename: string,
) {
	const state = view.state;
	const doc = state.doc;

	// Get raw text content - the editor stores markdown directly in the text
	const markdown = doc.textContent;

	downloadFile(markdown, filename, "text/markdown");
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
 * Exports the current editor content as a single-file HTML document.
 * Converts the markdown text to HTML using the marked parser.
 */
export function exportAsHtml(
	view: EditorView,
	filename: string,
	title: string,
) {
	const state = view.state;
	const doc = state.doc;
	const markdown = doc.textContent;
	const tokens = md.lexer(markdown);
	const htmlContent = md.parser(tokens);
	const fullHtml = generateHtmlDocument(title, htmlContent);
	downloadFile(fullHtml, filename, "text/html");
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
