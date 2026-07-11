import { createSignal } from "solid-js";

// TODO: implement this

type DndItem = { type: "room"; id: string } | { type: "folder"; id: string };

type DndTarget = {
	id: string;
	mode: "before" | "after" | "inside";
};

export const useRoomDnd = () => {
	const [dragging, setDragging] = createSignal<DndItem | null>(null);
	const [target, setTarget] = createSignal<DndTarget | null>(null);

	const handle = (e: DragEvent) => {
		e.preventDefault();
		e.stopPropagation();

		if (e.dataTransfer) {
			e.dataTransfer.effectAllowed = "move";
		}

		switch (e.type) {
			case "dragstart": {
				// TODO: handle
				break;
			}
			case "dragover": {
				// TODO: handle add room to folder
				// TODO: handle create new folder
				break;
			}
			case "dragend": {
				setTarget(null);
				setDragging(null);
				break;
			}
			case "drop": {
				// TODO: handle reorder room
				// TODO: handle add room to folder
				// TODO: handle create new folder
				break;
			}
		}
	};

	return {
		// TODO
		handle,
	};
};
