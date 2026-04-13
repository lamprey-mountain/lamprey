// NOTE: a lot of these are rendered with solidjs, so toDOM/parseDOM could probably be simplified?

import { Schema } from "prosemirror-model";

type FilterNodeSpec = {
	group: string;
	inline: boolean;
	atom: boolean;
	attrs: Record<string, { default: string | boolean }>;
	toDOM: (
		node: import("prosemirror-model").Node,
	) => import("prosemirror-model").DOMOutputSpec;
	parseDOM: Array<{
		tag: string;
		getAttrs: (dom: HTMLElement) => Record<string, unknown>;
	}>;
};

const createFilterNode = (
	name: string,
	valueKey: "id" | "value" | "date" = "value",
	hasNameAttr: boolean = false,
): FilterNodeSpec => ({
	group: "inline",
	inline: true,
	atom: true,
	attrs: {
		[valueKey]: { default: "" },
		...(hasNameAttr ? { name: { default: "" } } : {}),
		negated: { default: false },
	},
	toDOM: (node: import("prosemirror-model").Node) => {
		const displayValue = hasNameAttr ? node.attrs.name : node.attrs[valueKey];
		return [
			"span",
			{
				class: `filter-${name} filter-atom${node.attrs.negated ? " filter-negated" : ""}`,
				"data-node-view-placeholder": "true",
				...(valueKey === "id" ? { "data-id": node.attrs.id } : {}),
			},
		];
	},
	parseDOM: [
		{
			tag: `span.filter-${name}`,
			getAttrs: (dom: HTMLElement) => ({
				[valueKey]:
					valueKey === "id"
						? dom.dataset.id
						: (dom.querySelector(".filter-value")?.textContent ?? ""),
				...(hasNameAttr
					? { name: dom.querySelector(".filter-value")?.textContent ?? "" }
					: {}),
				negated: dom.classList.contains("filter-negated"),
			}),
		},
	],
});

export const schema = new Schema({
	nodes: {
		doc: { content: "paragraph" },
		paragraph: {
			content: "inline*",
			group: "block",
			toDOM: () => ["p", 0],
		},
		text: { group: "inline" },
		author: createFilterNode("author", "id", true),
		channel: createFilterNode("channel", "id", true),
		before: createFilterNode("before", "date"),
		after: createFilterNode("after", "date"),
		has: createFilterNode("has", "value"),
		pinned: createFilterNode("pinned", "value"),
		mentions: createFilterNode("mentions", "id", true),
	},
});
