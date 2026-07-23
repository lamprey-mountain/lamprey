import { type Accessor, createEffect, createSignal, onCleanup } from "solid-js";
import { createKeybinds } from "@/lib/keybinds";

export type RoomNavItemType = "home" | "folder" | "room" | "view";

export type RoomNavFocusItem = {
	id: string;
	type: RoomNavItemType;
	folderId: string | null;
};

export type RoomKeyboardListProps = {
	items: Accessor<RoomNavFocusItem[]>;
	selectedId: Accessor<string | null>;
	onToggleFolder: (id: string) => void;
};

export const useRoomNavKeybinds = (props: RoomKeyboardListProps) => {
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

	const getBlocks = () => {
		const blocks: { folderId: string | null; items: RoomNavFocusItem[] }[] = [];
		let currentBlock: RoomNavFocusItem[] = [];
		let currentFolderId: string | null = null;

		const allItems = props.items();
		for (const item of allItems) {
			if (item.folderId !== currentFolderId) {
				if (currentFolderId !== null) {
					blocks.push({ folderId: currentFolderId, items: currentBlock });
				}
				currentFolderId = item.folderId;
				currentBlock = [];
			}
			currentBlock.push(item);
		}
		if (currentFolderId !== null) {
			blocks.push({ folderId: currentFolderId, items: currentBlock });
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

			if (currentIndex + 1 < items.length) {
				focusItem(items[currentIndex + 1].id);
			}
		},
		ArrowUp: (e) => {
			e.preventDefault();
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const currentIndex = items.findIndex((i) => i.id === currentId);
			const current = items[currentIndex];
			if (!current) return;

			if (currentIndex - 1 >= 0) {
				focusItem(items[currentIndex - 1].id);
			}
		},
		PageDown: (e) => {
			e.preventDefault();
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const currentIndex = items.findIndex((i) => i.id === currentId);
			const current = items[currentIndex];
			if (!current) return;

			const blocks = getBlocks();
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
		},
		PageUp: (e) => {
			e.preventDefault();
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const currentIndex = items.findIndex((i) => i.id === currentId);
			const current = items[currentIndex];
			if (!current) return;

			const blocks = getBlocks();
			const blockIdx = blocks.findIndex((b) =>
				b.items.some((i) => i.id === current.id),
			);
			if (blockIdx !== -1) {
				const block = blocks[blockIdx];
				if (!block.items.length) return;

				const targetItem = block.items[0];
				if (current.id !== targetItem.id) {
					focusItem(targetItem.id);
				} else if (blockIdx - 1 >= 0) {
					const prevBlock = blocks[blockIdx - 1];
					const prevTarget = prevBlock.items[0];
					focusItem(prevTarget.id);
				}
			}
		},
		"Enter, Space": (e) => {
			e.preventDefault();
			const items = props.items();
			const currentId = focusedId() ?? props.selectedId();
			const current = items.find((i) => i.id === currentId);
			if (!current) return;

			if (current.type === "folder") {
				props.onToggleFolder(current.id);
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
