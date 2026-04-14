import type { ReferenceElement } from "@floating-ui/dom";
import type { Channel, User } from "sdk";
import {
	batch,
	createContext,
	type ParentComponent,
	useContext,
} from "solid-js";
import { createStore } from "solid-js/store";

export type AutocompleteState = {
	visible: boolean;
	reference: ReferenceElement | null;
	query: string;
	kind: AutocompleteKind | null;
	items: AutocompleteItem[]; // The filtered results
	activeIndex: number;
};

export type AutocompleteContext = {
	state: AutocompleteState;
	show: (reference: ReferenceElement, kind: AutocompleteKind) => void;
	hide: () => void;
	updateQuery: (query: string) => void;
	setReference: (ref: ReferenceElement) => void;
	setIndex: (index: number) => void;
	setResults: (items: AutocompleteItem[]) => void;
	navigate: (direction: "up" | "down") => void;
	select: () => void;
};

export type AutocompleteKind =
	| {
			type: "mention";
			onSelect: (item: AutocompleteMentionItem) => void;
			channelId: string;
			roomId?: string;
	  }
	| {
			type: "channel";
			onSelect: (item: Extract<AutocompleteItem, { type: "channel" }>) => void;
			channelId: string;
	  }
	| {
			type: "emoji";
			onSelect: (id: string, name: string, char?: string) => void;
			channelId: string;
	  }
	| {
			type: "command";
			onSelect: (command: string) => void;
			channelId: string;
	  };

export type AutocompleteMentionItem =
	| { type: "user"; user_id: string; name: string; user: User }
	| { type: "role"; role_id: string; name: string }
	| { type: "everyone"; mention_type: "everyone" };

export type AutocompleteItem =
	| AutocompleteMentionItem
	| { type: "channel"; channel: Channel; channel_id: string; name: string }
	| { type: "emoji"; id: string; name: string; char?: string }
	| { type: "command"; command: string; id: string; description?: string };

function isMentionKind(
	kind: AutocompleteKind | null,
): kind is Extract<AutocompleteKind, { type: "mention" }> {
	return kind?.type === "mention";
}

function isChannelKind(
	kind: AutocompleteKind | null,
): kind is Extract<AutocompleteKind, { type: "channel" }> {
	return kind?.type === "channel";
}

function isEmojiKind(
	kind: AutocompleteKind | null,
): kind is Extract<AutocompleteKind, { type: "emoji" }> {
	return kind?.type === "emoji";
}

function isCommandKind(
	kind: AutocompleteKind | null,
): kind is Extract<AutocompleteKind, { type: "command" }> {
	return kind?.type === "command";
}

const AutocompleteContext = createContext<AutocompleteContext>();

export const AutocompleteProvider: ParentComponent = (props) => {
	const [state, update] = createStore<AutocompleteState>({
		visible: false,
		reference: null,
		query: "",
		kind: null,
		items: [],
		activeIndex: 0,
	});

	const show = (reference: ReferenceElement, kind: AutocompleteKind) => {
		batch(() => {
			update("reference", reference);
			update("kind", kind);
			update("visible", true);
		});
	};

	const hide = () => {
		batch(() => {
			update("visible", false);
			update("kind", null);
			update("items", []);
			update("query", "");
			update("activeIndex", 0);
		});
	};

	const navigate = (direction: "up" | "down") => {
		const len = state.items.length;
		if (len === 0) return;
		const offset = direction === "up" ? -1 : 1;
		update("activeIndex", (i) => (i + offset + len) % len);
	};

	const select = () => {
		const item = state.items[state.activeIndex];
		if (item && state.kind) {
			const kind = state.kind;
			if (
				isMentionKind(kind) &&
				(item.type === "user" ||
					item.type === "role" ||
					item.type === "everyone")
			) {
				kind.onSelect(item);
			} else if (isChannelKind(kind) && item.type === "channel") {
				kind.onSelect(item);
			} else if (isEmojiKind(kind) && item.type === "emoji") {
				kind.onSelect(item.id, item.name, item.char);
			} else if (isCommandKind(kind) && item.type === "command") {
				kind.onSelect(item.command);
			}
			hide();
		}
	};

	const context: AutocompleteContext = {
		state,
		show,
		hide,
		navigate,
		select,
		updateQuery: (query) =>
			batch(() => {
				update("query", query);
				update("activeIndex", 0);
			}),
		setReference: (reference) => update("reference", reference),
		setIndex: (index) => update("activeIndex", index),
		setResults: (items) => {
			batch(() => {
				update("items", items);
				// Clamp index if list shrunk
				if (state.activeIndex >= items.length) {
					update("activeIndex", 0);
				}
			});
		},
	};

	return (
		<AutocompleteContext.Provider value={context}>
			{props.children}
		</AutocompleteContext.Provider>
	);
};

export const useAutocomplete = () => {
	const context = useContext(AutocompleteContext);
	if (!context) {
		throw new Error(
			"useAutocomplete must be used within an AutocompleteProvider",
		);
	}
	return context;
};
