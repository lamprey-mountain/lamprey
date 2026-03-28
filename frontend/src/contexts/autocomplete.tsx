import { ReferenceElement } from "@floating-ui/dom";
import {
	type Accessor,
	batch,
	createContext,
	createSignal,
	type ParentComponent,
	type Setter,
	useContext,
} from "solid-js";
import { createStore, SetStoreFunction } from "solid-js/store";
import type { User } from "sdk";

export type AutocompleteState = {
	visible: boolean;
	reference: ReferenceElement | null;
	query: string;
	kind: AutocompleteKind | null;
	items: any[]; // The filtered results
	activeIndex: number;
};

export type AutocompleteContext = {
	state: AutocompleteState;
	show: (reference: ReferenceElement, kind: AutocompleteKind) => void;
	hide: () => void;
	updateQuery: (query: string) => void;
	setReference: (ref: ReferenceElement) => void;
	setIndex: (index: number) => void;
	setResults: (items: any[]) => void;
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
		onSelect: (channelId: string, channelName: string) => void;
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
			(state.kind.onSelect as any)(item as any);
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
