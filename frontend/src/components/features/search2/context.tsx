import { type NodeSpec, Schema } from "prosemirror-model";

export type SearchConfig = {
	schema: Schema;
	// TODO: what else goes here?
};

export type Filter = {
	name: string;

	// /** The filter keyword (e.g. "author", "channel") */
	// name: string;
	// /** What kind of value the filter expects */
	// valueType: SearchFilterValueType;
	// /** Whether the node stores a human-readable name in addition to the value */
	// hasNameAttr: boolean;
	// /** Produce suggestion items given the current partial query */
	// getSuggestions: (query: string, ctx: SearchContext) => SuggestionItem[];
	// resolveDisplayData?: (
	// 	value: string,
	// 	ctx: SearchContext,
	// ) => { name?: string; user?: User; channel?: ThreadT };
	// /** Convert a ProseMirror node → our intermediate FilterASTNode */
	// toAST: (node: Node) => FilterASTNode;
	// /** Convert a FilterASTNode → the backend query string fragment */
	// toBackendQuery: (node: FilterASTNode) => string[];
	// /** Convert a FilterASTNode → a ProseMirror node */
	// toPMNode: (ast: FilterASTNode) => Node;
};

export type CreateSearchOptions = {
	filters: Filter[];
};

export const createSearch = (options: CreateSearchOptions): SearchConfig => {
	const schema = new Schema({
		nodes: {
			query: {
				content: "inline*",
				group: "block",
				toDOM: () => ["p", 0],
			},
			text: { group: "inline" },
			...Object.fromEntries(
				options.filters.map((f) => [
					`filter-${f.name}`,
					{
						group: "inline",
						inline: true,
						atom: true,
						attrs: {
							id: {
								// default: "",
								validate: "string",
							},
						},
						// leafText(node) {
						//   return node.attrs.name
						// },
						toDOM: (node) => [
							"span",
							{
								class: "filter",
								"data-filter-name": f.name,
							},
							// "custom text?",
							0,
						],
						parseDOM: [
							{
								tag: `span.filter[data-filter-name=${f.name}]`,
								getAttrs: (el) => ({
									id: el.dataset.userId,
								}),
							},
						],
					} as NodeSpec,
				]),
			),
			// author: createFilterNode("author", "id", true),
			// channel: createFilterNode("channel", "id", true),
			// before: createFilterNode("before", "date"),
			// after: createFilterNode("after", "date"),
			// has: createFilterNode("has", "value"),
			// pinned: createFilterNode("pinned", "value"),
			// mentions: createFilterNode("mentions", "id", true),
		},
		topNode: "query",
	});

	return {
		schema,
	};
};
