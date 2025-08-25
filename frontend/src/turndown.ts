// some gfm stuff is vendored because the npm package doesn't have types

import TurndownService from "turndown";

const highlightRegExp =
	/\b(?:language|lang|source|highlight(?:-source|-lang)?)-([a-z0-9_+-]+)\b/i;

const languageMap: Record<string, string> = {
	"html-basic": "html",
	"text-html-basic": "html",
	"source-js": "javascript",
	"text-javascript": "javascript",
	"source-python": "python",
	"source-css": "css",
	"source-shell": "bash",
	"source-json": "json",
	"source-yaml": "yaml",
	"source-xml": "xml",
};

function extractLanguage(node: Element): string {
	// HACK: try to read language from the parent node
	return extractLanguageInner(node) ||
		(node.parentElement ? extractLanguageInner(node.parentElement) : "");
}

function extractLanguageInner(node: Element): string {
	const className = node.className || "";
	const language = (className.match(highlightRegExp) || [null, ""])[1] ||
		"";
	return languageMap[language] || language;
}

function highlightedCodeBlock(turndownService: TurndownService) {
	turndownService.addRule("highlightedCodeBlock", {
		filter: (node: HTMLElement) => {
			return node.matches("pre") ||
				(highlightRegExp.test(node.className) &&
					!!node.querySelector(":scope > pre"));
		},
		replacement: (content, node, options) => {
			if (!(node instanceof HTMLElement)) return content;
			const pre = node.nodeName === "PRE"
				? node
				: node.querySelector(":scope > pre");
			const code = pre?.querySelector(":scope > code") ??
				node.querySelector(":scope > code");
			const lang = extractLanguage(node) ||
				(pre ? extractLanguage(pre) : "") ||
				(code ? extractLanguage(code) : "");
			const text = (pre?.textContent ?? node.textContent ?? "")
				.replace(/\r\n?/g, "\n");
			return `\n\n${options.fence}${lang}\n${text}\n${options.fence}\n\n`;
		},
	});
}

function strikethrough(turndownService: TurndownService) {
	turndownService.addRule("strikethrough", {
		filter: ["del", "s", "strike" as any],
		replacement: (content: string) => `~~${content}~~`,
	});
}

function taskListItems(turndownService: TurndownService) {
	turndownService.addRule("taskListItems", {
		filter: (node: Node) => {
			return node instanceof HTMLInputElement &&
				node.type === "checkbox" &&
				node.parentNode instanceof HTMLLIElement;
		},
		replacement: (_content: string, node: Node) => {
			if (!(node instanceof HTMLInputElement)) return "";
			return node.checked ? "[x] " : "[ ] ";
		},
	});
}

function tableCellNormalize(
	content: string,
	node: HTMLTableCellElement,
	_isHeader = false, // unused?
) {
	const parentElement = node.parentElement;
	if (!parentElement) return `| ${content.trim()} |`;

	const cellIndex = Array.from(parentElement.children).indexOf(node);
	const prefix = cellIndex === 0 ? "| " : " ";
	return `${prefix}${content.trim()} |`;
}

function isHeadingRow(tr?: HTMLTableRowElement) {
	if (!tr) return false;

	const parentNode = tr.parentNode as HTMLElement;
	return (
		parentNode.nodeName === "THEAD" ||
		(
			parentNode.firstChild === tr &&
			(parentNode.nodeName === "TABLE" || isFirstTbody(parentNode)) &&
			Array.from(tr.cells).every((cell) => cell.nodeName === "TH")
		)
	);
}

function isFirstTbody(element: HTMLElement) {
	const previousSibling = element.previousSibling as HTMLElement | null;
	return (
		element.nodeName === "TBODY" &&
		(!previousSibling ||
			(previousSibling.nodeName === "THEAD" &&
				/^\s*$/i.test(previousSibling.textContent || "")))
	);
}

function tables(turndownService: TurndownService) {
	turndownService.keep((node: HTMLElement) => {
		if (node.nodeName !== "TABLE") return false;
		const table = node as HTMLTableElement;
		return table.rows.length === 0 || !isHeadingRow(table.rows[0]);
	});

	turndownService.addRule("tableCell", {
		filter: ["th", "td"],

		replacement: (content: string, node: Node) => {
			if (!(node instanceof HTMLTableCellElement)) return content;
			return tableCellNormalize(content, node);
		},
	});

	turndownService.addRule("tableRow", {
		filter: "tr",

		replacement: (content: string, node: Node) => {
			if (!(node instanceof HTMLTableRowElement)) return content;

			const cells = Array.from(node.cells);
			const row = cells.map((cellNode) =>
				tableCellNormalize(cellNode.textContent || "", cellNode)
			).join("");
			if (isHeadingRow(node)) {
				const alignMap: Record<string, string> = {
					left: ":--",
					right: "--:",
					center: ":-:",
				};
				const borderCells = Array.from(node.cells)
					.map((c) => {
						const align = (c.getAttribute("align") || "").toLowerCase();
						return alignMap[align] ?? "---";
					})
					.map((border, i) => (i === 0 ? `| ${border} |` : ` ${border} |`))
					.join("");
				return `\n${row}\n${borderCells}`;
			}
			return `\n${row}`;
		},
	});

	turndownService.addRule("table", {
		filter: "table",
		replacement: (content: string) => `\n\n${content}\n\n`,
	});

	turndownService.addRule("tableSection", {
		filter: ["thead", "tbody", "tfoot"],
		replacement: (content: string) => content,
	});

	turndownService.addRule("tableCaption", {
		filter: "caption",
		replacement: (content: string) => `\n\n**${content.trim()}**\n`,
	});
}

export function initTurndownService(): TurndownService {
	const turndownService = new TurndownService({
		headingStyle: "atx",
		hr: "---",
		bulletListMarker: "-",
		codeBlockStyle: "fenced",
		emDelimiter: "*",
		strongDelimiter: "**",
		linkStyle: "inlined",
		linkReferenceStyle: "full",
	});

	turndownService.use([
		highlightedCodeBlock,
		strikethrough,
		tables,
		taskListItems,
	]);

	// people sometimes like to add "link to paragraph" things
	turndownService.addRule("skipEmptyLinks", {
		filter: (node) => node.nodeName === "A" && !node.textContent?.trim(),
		replacement: () => "",
	});

	// fix spacing
	turndownService.addRule("lineBreak", {
		filter: "br",
		replacement: () => "\n",
	});

	return turndownService;
}
