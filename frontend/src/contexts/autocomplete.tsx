import { ReferenceElement } from "@floating-ui/dom";
import {
	type Accessor,
	createContext,
	createSignal,
	type ParentComponent,
	type Setter,
	useContext,
} from "solid-js";

export type AutocompleteState =
	| {
		type: "mention";
		query: string;
		ref: ReferenceElement;
		onSelect: (userId: string, userName: string) => void;
		channelId: string;
	}
	| {
		type: "channel";
		query: string;
		ref: ReferenceElement;
		onSelect: (channelId: string, channelName: string) => void;
		channelId: string;
	}
	| {
		type: "emoji";
		query: string;
		ref: ReferenceElement;
		onSelect: (id: string, name: string, char?: string) => void;
		channelId: string;
	}
	| {
		type: "command";
		query: string;
		ref: ReferenceElement;
		onSelect: (command: string) => void;
		channelId: string;
	}
	| null;

export type AutocompleteContextT = {
	autocomplete: Accessor<AutocompleteState>;
	setAutocomplete: Setter<AutocompleteState>;
};

const AutocompleteContext = createContext<AutocompleteContextT>();

export const AutocompleteProvider: ParentComponent = (props) => {
	const [autocomplete, setAutocomplete] = createSignal<AutocompleteState>(null);

	return (
		<AutocompleteContext.Provider value={{ autocomplete, setAutocomplete }}>
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
