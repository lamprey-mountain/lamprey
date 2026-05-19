import {
	createContext,
	createSignal,
	type ParentProps,
	useContext,
} from "solid-js";
import type { Script } from "ts-sdk";

type ScriptContextT = {
	channel_id: string;
	script?: Script;
	root?: ScriptPane;

	createPane(create: ScriptPaneCreate): void;
	closePane(tab_id: number): void;
	updatePaneSize(tab_id: number, size: number): void;

	/** close all tabs */
	reset(): void;

	/** replace all existing tabs to point to a different script */
	switchScript(script_id: string): void;
};

export type ScriptPane = ScriptPaneType & {
	id: number;
};

export type ScriptPaneChild = ScriptPane & {
	/** the size of this pane in pixels, otherwise flex: 1 */
	size?: number;
};

export type ScriptPaneType =
	| { type: "split_horizontal"; children: ScriptPaneChild[] }
	| { type: "split_vertical"; children: ScriptPaneChild[] }
	| { type: "script_code"; script_id: string }
	| { type: "script_inputs"; script_id: string }
	| { type: "script_preview"; script_id: string }
	| { type: "run_logs"; script_id: string; run_id: string };
// future: run_traces (needs api design and backend support first)

export type ScriptPaneCreate = (
	| { type: "split_horizontal" }
	| { type: "split_vertical" }
	| { type: "script_code"; script_id: string }
	| { type: "script_inputs"; script_id: string }
	| { type: "script_preview"; script_id: string }
	| { type: "run_logs"; script_id: string; run_id: string }
) & {
	/** unique identifier for this pane, if empty create one automatically */
	id?: number;

	parentId?: number;
};

export const ScriptContext = createContext<ScriptContextT>();

// maybe don't use a global counter? this is probably fine though.
let nextPaneId = 1;
const assignTabId = () => nextPaneId++;

const _findParent = (
	root: ScriptPane,
	parentId: number,
): ScriptPane | undefined => {
	if (root.id === parentId) return root;
	const children =
		root.type === "split_horizontal" || root.type === "split_vertical"
			? root.children
			: [];
	for (const child of children) {
		const found = _findParent(child, parentId);
		if (found) return found;
	}
	return undefined;
};

const addChildToParent = (
	root: ScriptPane,
	parentId: number,
	child: ScriptPane,
): ScriptPane => {
	if (root.id === parentId) {
		const type = root.type;
		if (type === "split_horizontal" || type === "split_vertical") {
			return {
				...root,
				children: [...root.children, child],
			};
		}
	}
	const type = root.type;
	if (type === "split_horizontal" || type === "split_vertical") {
		return {
			...root,
			children: root.children.map((c) => addChildToParent(c, parentId, child)),
		};
	}
	return root;
};

const removeTab = (root: ScriptPane, tabId: number): ScriptPane | null => {
	if (root.id === tabId) return null;
	const type = root.type;
	if (type === "split_horizontal" || type === "split_vertical") {
		const newChildren = root.children
			.map((c) => removeTab(c, tabId))
			.filter((c): c is ScriptPane => c !== null);
		if (newChildren.length === 0) return null;
		if (newChildren.length === 1) return newChildren[0];
		return { ...root, children: newChildren };
	}
	return root;
};

const _removeChildByParent = (
	root: ScriptPane,
	parentId: number,
	tabId: number,
): ScriptPane => {
	if (root.id === parentId) {
		const type = root.type;
		if (type === "split_horizontal" || type === "split_vertical") {
			const newChildren = root.children
				.map((c) => removeTab(c, tabId))
				.filter((c): c is ScriptPane => c !== null);
			if (newChildren.length === 0) {
				return {
					...root,
					children: [] as ScriptPaneChild[],
				};
			}
			if (newChildren.length === 1) return newChildren[0];
			return { ...root, children: newChildren };
		}
	}
	const type = root.type;
	if (type === "split_horizontal" || type === "split_vertical") {
		return {
			...root,
			children: root.children.map((c) =>
				_removeChildByParent(c, parentId, tabId),
			),
		};
	}
	return root;
};

const _replaceTab = (
	root: ScriptPane,
	tabId: number,
	replacement: ScriptPane,
): ScriptPane => {
	if (root.id === tabId) return replacement;
	const type = root.type;
	if (type === "split_horizontal" || type === "split_vertical") {
		return {
			...root,
			children: root.children.map((c) => _replaceTab(c, tabId, replacement)),
		};
	}
	return root;
};

export const createScriptContext = (channel_id: string) => {
	const [root, setRoot] = createSignal<ScriptPane | undefined>();

	const ctx: ScriptContextT = {
		channel_id,
		get root() {
			return root();
		},

		createPane(create: ScriptPaneCreate) {
			const tabId = create.id ?? assignTabId();
			const tab: ScriptPane = {
				id: tabId,
				...create,
				...(create.type === "split_horizontal" ||
				create.type === "split_vertical"
					? { children: [] }
					: {}),
			} as ScriptPane;
			setRoot((prev) => {
				if (!prev) return tab;
				if (create.parentId === undefined) return tab;
				return addChildToParent(prev, create.parentId, tab);
			});
		},

		closePane(tabId) {
			setRoot((prev) => {
				if (!prev) return undefined;
				const result = removeTab(prev, tabId);
				return result ?? undefined;
			});
		},

		updatePaneSize(tabId, size) {
			setRoot((prev) => {
				if (!prev) return undefined;
				const resize = (node: ScriptPane): ScriptPane => {
					if (node.id === tabId) {
						return { ...node, size } as unknown as ScriptPane;
					}
					if (
						node.type === "split_horizontal" ||
						node.type === "split_vertical"
					) {
						return {
							...node,
							children: node.children.map((c) => resize(c as ScriptPane)),
						};
					}
					return node;
				};
				return resize(prev);
			});
		},

		reset() {
			setRoot(undefined);
		},

		switchScript(scriptId) {
			setRoot((prev) => {
				if (!prev) return prev;
				// Find the pane with this script_id and replace it with script_code
				const findAndReplace = (node: ScriptPane): ScriptPane => {
					if (
						node.type === "script_code" ||
						node.type === "script_inputs" ||
						node.type === "script_preview"
					) {
						if (node.script_id === scriptId) {
							return {
								type: "script_code",
								script_id: scriptId,
								id: node.id,
							} as ScriptPane;
						}
						return node;
					}
					if (
						node.type === "split_horizontal" ||
						node.type === "split_vertical"
					) {
						return {
							...node,
							children: node.children.map((c) =>
								findAndReplace(c as ScriptPane),
							),
						};
					}
					return node;
				};
				return findAndReplace(prev);
			});
		},
	};

	return ctx;
};

export const useScript = () => {
	const ctx = useContext(ScriptContext);
	if (!ctx) {
		throw new Error("useScript must be used within a ScriptContext.Provider");
	}
	return ctx;
};
