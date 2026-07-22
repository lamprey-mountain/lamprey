import { type Accessor, createEffect, createSignal, onCleanup } from "solid-js";
import { createKeybinds } from "@/lib/keybinds";

export type NavItemType = "category" | "channel" | "thread";

export type NavItem = {
	id: string;
	type: NavItemType;
	categoryId: string | null;
	channelId: string | null;
	hasThreads?: boolean;
};

export type KeyboardListProps = {
	items: Accessor<NavItem[]>;
	categories: Accessor<any>;
	selectedId: Accessor<string | null>;
	onToggleCategory: (id: string) => void;
	onSelectChannel: (id: string) => void;
};

export const useChannelNavKeybinds = (props: KeyboardListProps) => {
	let containerEl: HTMLElement | undefined;
	const [focusedId, setFocusedId] = createSignal<string | null>(null);

	const onFocusOut = (e: FocusEvent) => {
		if (containerEl && !containerEl.contains(e.relatedTarget as Node)) {
			setFocusedId(props.selectedId());
		}
	};

	createEffect(() => {
		const id = props.selectedId();
		if (id && (!containerEl || !containerEl.contains(document.activeElement))) {
			setFocusedId(id);
		}
	});

	const focusItem = (id: string) => {
		setFocusedId(id);
		setTimeout(() => {
			if (!containerEl) return;
			const el = containerEl.querySelector(
				`[data-nav-id="${id}"]`,
			) as HTMLElement;
			if (el) el.focus();
		}, 0);
	};

	const getVisibleCategoryBlocks = () => {
		const blocks: { categoryId: string | null; items: NavItem[] }[] = [];
		let currentBlock: NavItem[] = [];
		let currentCatId: string | null = undefined as any;

		const allItems = props.items();
		for (const item of allItems) {
			if (item.type === "thread") continue;

			if (item.categoryId !== currentCatId) {
				if (currentCatId !== undefined) {
					blocks.push({ categoryId: currentCatId, items: currentBlock });
				}
				currentCatId = item.categoryId;
				currentBlock = [];
			}
			currentBlock.push(item);
		}
		if (currentCatId !== undefined) {
			blocks.push({ categoryId: currentCatId, items: currentBlock });
		}
		return blocks;
	};

	const binds = createKeybinds({
		ArrowDown: (e) => {
			e.preventDefault();
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const currentIndex = items.findIndex((i) => i.id === currentId);
			const current = items[currentIndex];
			if (!current) {
				if (items.length > 0) focusItem(items[0].id);
				return;
			}

			if (current.type === "thread") {
				const nextThread = items.findIndex(
					(i, idx) =>
						idx > currentIndex &&
						i.type === "thread" &&
						i.channelId === current.channelId,
				);
				if (nextThread !== -1) focusItem(items[nextThread].id);
			} else {
				const nextMain = items.findIndex(
					(i, idx) =>
						idx > currentIndex &&
						(i.type === "category" || i.type === "channel"),
				);
				if (nextMain !== -1) focusItem(items[nextMain].id);
			}
		},
		ArrowUp: (e) => {
			e.preventDefault();
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const currentIndex = items.findIndex((i) => i.id === currentId);
			const current = items[currentIndex];
			if (!current) return;

			if (current.type === "thread") {
				const prevThreadIdx = items.findLastIndex(
					(i, idx) =>
						idx < currentIndex &&
						i.type === "thread" &&
						i.channelId === current.channelId,
				);
				if (prevThreadIdx !== -1) focusItem(items[prevThreadIdx].id);
			} else {
				const prevMainIdx = items.findLastIndex(
					(i, idx) =>
						idx < currentIndex &&
						(i.type === "category" || i.type === "channel"),
				);
				if (prevMainIdx !== -1) focusItem(items[prevMainIdx].id);
			}
		},
		ArrowRight: (e) => {
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const currentIndex = items.findIndex((i) => i.id === currentId);
			const current = items[currentIndex];
			if (!current) return;

			if (current.type === "channel" && current.hasThreads) {
				e.preventDefault();
				const firstThreadIdx = items.findIndex(
					(i, idx) =>
						idx > currentIndex &&
						i.type === "thread" &&
						i.channelId === current.channelId,
				);
				if (firstThreadIdx !== -1) focusItem(items[firstThreadIdx].id);
			}
		},
		ArrowLeft: (e) => {
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const currentIndex = items.findIndex((i) => i.id === currentId);
			const current = items[currentIndex];
			if (!current) return;

			if (current.type === "thread") {
				e.preventDefault();
				const parentChannelIdx = items.findIndex(
					(i) => i.id === current.channelId,
				);
				if (parentChannelIdx !== -1) focusItem(items[parentChannelIdx].id);
			}
		},
		PageDown: (e) => {
			e.preventDefault();
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const currentIndex = items.findIndex((i) => i.id === currentId);
			const current = items[currentIndex];
			if (!current) return;

			if (current.type === "thread") {
				const lastThreadIdx = items.findLastIndex(
					(i) => i.type === "thread" && i.channelId === current.channelId,
				);
				if (lastThreadIdx !== -1 && lastThreadIdx !== currentIndex) {
					focusItem(items[lastThreadIdx].id);
				}
			} else {
				const blocks = getVisibleCategoryBlocks();
				const blockIdx = blocks.findIndex((b) =>
					b.items.some((i) => i.id === current.id),
				);
				if (blockIdx !== -1) {
					const block = blocks[blockIdx];
					if (!block.items.length) return;
					const lastItem = block.items[block.items.length - 1];
					if (current.id !== lastItem.id) {
						focusItem(lastItem.id);
					} else if (blockIdx + 1 < blocks.length) {
						const nextBlock = blocks[blockIdx + 1];
						focusItem(nextBlock.items[nextBlock.items.length - 1].id);
					}
				}
			}
		},
		PageUp: (e) => {
			e.preventDefault();
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const currentIndex = items.findIndex((i) => i.id === currentId);
			const current = items[currentIndex];
			if (!current) return;

			if (current.type === "thread") {
				const firstThreadIdx = items.findIndex(
					(i) => i.type === "thread" && i.channelId === current.channelId,
				);
				if (firstThreadIdx !== -1 && firstThreadIdx !== currentIndex) {
					focusItem(items[firstThreadIdx].id);
				}
			} else {
				const blocks = getVisibleCategoryBlocks();
				const blockIdx = blocks.findIndex((b) =>
					b.items.some((i) => i.id === current.id),
				);
				if (blockIdx !== -1) {
					const block = blocks[blockIdx];
					if (!block.items.length) return;

					const findFirstChannel = (items: NavItem[]) =>
						items.find((i) => i.type === "channel") || items[0];

					const targetItem = findFirstChannel(block.items);
					if (current.id !== targetItem.id) {
						focusItem(targetItem.id);
					} else if (blockIdx - 1 >= 0) {
						const prevBlock = blocks[blockIdx - 1];
						const prevTarget = findFirstChannel(prevBlock.items);
						focusItem(prevTarget.id);
					}
				}
			}
		},
		"Enter, Space": (e) => {
			e.preventDefault();
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const current = items.find((i) => i.id === currentId);
			if (!current) return;

			if (current.type === "category") {
				props.onToggleCategory(current.id);
			} else {
				if (containerEl) {
					const el = containerEl.querySelector(`[data-nav-id="${current.id}"]`);
					if (el) {
						const link = el.querySelector("a");
						if (link) {
							(link as HTMLElement).click();
						} else {
							(el as HTMLElement).click();
						}
					}
				}
			}
		},
	});

	const container = (el: HTMLElement) => {
		containerEl = el;
		el.addEventListener("keydown", binds);
		el.addEventListener("focusout", onFocusOut);
		onCleanup(() => {
			el.removeEventListener("keydown", binds);
			el.removeEventListener("focusout", onFocusOut);
		});
	};

	return {
		container,
		focusedId,
	};
};
