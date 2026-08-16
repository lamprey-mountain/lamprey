import {
	createContext,
	createSignal,
	For,
	type JSX,
	Match,
	type ParentProps,
	Show,
	Switch,
	useContext,
} from "solid-js";
import { PaneResizeHandle } from "@/atoms/Resizable";

export type PanePlacement = "top" | "bottom" | "left" | "right";

export type PaneNode<P> = {
	id: number;
	size?: number;
} & (
	| { type: "split_horizontal"; children: PaneNode<P>[] }
	| { type: "split_vertical"; children: PaneNode<P>[] }
	| { type: "leaf"; data: P }
);

export type PaneCreate<P> = {
	id?: number;
	parentId?: number;
} & (
	| { type: "split_horizontal" }
	| { type: "split_vertical" }
	| { type: "leaf"; data: P }
);

export type PaneDirection = "horizontal" | "vertical";

// TODO: add doc comments
export type PanesState<P> = {
	root: PaneNode<P> | undefined;

	create(create: PaneCreate<P>): void;
	close(id: number): void;
	resize(id: number, size: number): void;
	find(predicate: (node: PaneNode<P>) => boolean): PaneNode<P> | undefined;
	update(
		id: number,
		update: Partial<Omit<PaneNode<P>, "id" | "children">>,
	): void;
	split(
		targetId: number,
		newPane: PaneCreate<P>,
		direction: PaneDirection,
	): void;
	// TODO: rewrite split(targetId: number, newPane: PaneCreate<P>, placement: PanePlacement): void;
	// TODO: add move(targetId: number, parentId: number, placement: PanePlacement): void;

	/** close all panes */
	closeAll(): void;
};

export type TemplateProps<
	P,
	T extends P extends { type: string } ? P["type"] : never,
> = {
	type: T;
	children: (pane: PaneNode<P> & { data: P }) => JSX.Element;
};

export const PanesContext = createContext<PanesState<any>>();

let nextPaneId = 1;
const assignPaneId = () => nextPaneId++;

const addChildToParent = <P,>(
	root: PaneNode<P>,
	parentId: number,
	child: PaneNode<P>,
): PaneNode<P> => {
	if (root.id === parentId) {
		if (root.type === "split_horizontal" || root.type === "split_vertical") {
			return {
				...root,
				children: [...root.children, child],
			};
		}
	}
	if (root.type === "split_horizontal" || root.type === "split_vertical") {
		return {
			...root,
			children: root.children.map((c) => addChildToParent(c, parentId, child)),
		};
	}
	return root;
};

const removePane = <P,>(
	root: PaneNode<P>,
	paneId: number,
): PaneNode<P> | null => {
	if (root.id === paneId) return null;
	if (root.type === "split_horizontal" || root.type === "split_vertical") {
		const newChildren = root.children
			.map((c) => removePane(c, paneId))
			.filter((c): c is PaneNode<P> => c !== null);
		if (newChildren.length === 0) return null;
		if (newChildren.length === 1) return newChildren[0];
		return { ...root, children: newChildren };
	}
	return root;
};

export type PaneTemplateProps<P> = {
	pane: PaneNode<P> & { data: P };
	setHeaderExtra: (el: JSX.Element) => void;
};

export type PanesProps<P> = {
	types: Record<string, (props: PaneTemplateProps<P>) => JSX.Element>;
};

export function createPanes<P extends { type: string }>(props: PanesProps<P>) {
	const [root, setRoot] = createSignal<PaneNode<P> | undefined>();
	const templates = new Map<
		string,
		(props: PaneTemplateProps<P>) => JSX.Element
	>();
	for (const [type, template] of Object.entries(props.types)) {
		templates.set(type, template);
	}

	const data: PanesState<P> = {
		get root() {
			return root();
		},

		create(create) {
			const paneId = create.id ?? assignPaneId();
			const pane: PaneNode<P> = {
				id: paneId,
				...create,
				...(create.type === "split_horizontal" ||
				create.type === "split_vertical"
					? { children: [] }
					: {}),
			} as PaneNode<P>;
			setRoot((prev) => {
				if (!prev) return pane;
				if (create.parentId === undefined) return pane;
				return addChildToParent(prev, create.parentId, pane);
			});
		},

		close(id) {
			setRoot((prev) => {
				if (!prev) return undefined;
				const result = removePane(prev, id);
				return result ?? undefined;
			});
		},

		resize(id, size) {
			setRoot((prev) => {
				if (!prev) return undefined;
				const resize = (node: PaneNode<P>): PaneNode<P> => {
					if (node.id === id) {
						return { ...node, size };
					}
					if (
						node.type === "split_horizontal" ||
						node.type === "split_vertical"
					) {
						return {
							...node,
							children: node.children.map(resize),
						};
					}
					return node;
				};
				return resize(prev);
			});
		},

		find(predicate) {
			const find = (node: PaneNode<P>): PaneNode<P> | undefined => {
				if (predicate(node)) return node;
				if (
					node.type === "split_horizontal" ||
					node.type === "split_vertical"
				) {
					for (const child of node.children) {
						const found = find(child);
						if (found) return found;
					}
				}
				return undefined;
			};
			const r = root();
			return r ? find(r) : undefined;
		},

		update(id, update) {
			setRoot((prev) => {
				if (!prev) return prev;
				const updateNode = (node: PaneNode<P>): PaneNode<P> => {
					if (node.id === id) {
						return { ...node, ...update } as PaneNode<P>;
					}
					if (
						node.type === "split_horizontal" ||
						node.type === "split_vertical"
					) {
						return { ...node, children: node.children.map(updateNode) };
					}
					return node;
				};
				return updateNode(prev);
			});
		},

		split(targetId, newPane, direction) {
			const paneId = newPane.id ?? assignPaneId();
			const pane: PaneNode<P> = {
				id: paneId,
				...newPane,
				...(newPane.type === "split_horizontal" ||
				newPane.type === "split_vertical"
					? { children: [] }
					: {}),
			} as PaneNode<P>;
			const splitId = assignPaneId();

			setRoot((prev) => {
				if (!prev) return prev;
				const split = (node: PaneNode<P>): PaneNode<P> => {
					if (node.id === targetId) {
						return {
							id: splitId,
							type:
								direction === "horizontal"
									? "split_horizontal"
									: "split_vertical",
							children: [node, pane],
						} as PaneNode<P>;
					}
					if (
						node.type === "split_horizontal" ||
						node.type === "split_vertical"
					) {
						return { ...node, children: node.children.map(split) };
					}
					return node;
				};
				return split(prev);
			});
		},

		closeAll() {
			setRoot(undefined);
		},
	};

	const NodeView = (props: { node: PaneNode<P> }): JSX.Element => {
		const paneType = () => {
			if (props.node.type === "leaf") return props.node.data.type;
			return props.node.type;
		};

		return (
			<div
				class="pane-item"
				classList={{ sized: props.node.size !== undefined }}
				data-pane-type={paneType()}
				style={{
					"--size": props.node.size ? `${props.node.size}px` : undefined,
				}}
			>
				<Switch>
					<Match
						when={
							props.node.type === "split_horizontal" ||
							props.node.type === "split_vertical"
						}
					>
						<For each={props.node.children}>
							{(child, index) => (
								<>
									<Show when={index() > 0}>
										<PaneResizeHandle
											isHorizontal={props.node.type === "split_horizontal"}
											onResize={(sz) => {
												data.resize(props.node.children[index() - 1].id, sz);
											}}
										/>
									</Show>
									<NodeView node={child} />
								</>
							)}
						</For>
					</Match>
					<Match when={templates.get(paneType())}>
						{(template) => {
							const pane = props.node as PaneNode<P> & { data: P };
							const [headerExtra, setHeaderExtra] =
								createSignal<JSX.Element>(null);
							return (
								<>
									<header class="pane-header">
										<nav>
											{paneType().replace("script_", "").replace("_", " ")}
										</nav>
										<div class="title">Pane {pane.id}</div>
										{headerExtra()}
										<button
											type="button"
											class="close"
											onClick={() => data.close(pane.id)}
										>
											&times;
										</button>
									</header>
									<div class="pane-content">
										{/* TODO: better reactivity for this? */}
										{template()({ pane, setHeaderExtra })}
									</div>
								</>
							);
						}}
					</Match>
					<Match when={true}>no template!</Match>
				</Switch>
			</div>
		);
	};

	const RootView = (props: { placeholder?: JSX.Element }) => {
		return (
			<PanesContext.Provider value={data}>
				<div class="pane-container">
					<Show when={root()} fallback={props.placeholder}>
						{(root) => <NodeView node={root()} />}
					</Show>
				</div>
			</PanesContext.Provider>
		);
	};

	const Template = (props: TemplateProps<P, P["type"]>) => {
		templates.set(props.type, props.children as any);
		return null;
	};

	return {
		...data,
		Render: RootView,
		Template,
	};
}

export function usePanes<P>() {
	const ctx = useContext(PanesContext);
	if (!ctx) {
		throw new Error("usePanes must be used within a PanesProvider");
	}
	return ctx as PanesState<P>;
}
