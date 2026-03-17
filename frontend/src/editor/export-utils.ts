import type { EditorView } from "prosemirror-view";

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
