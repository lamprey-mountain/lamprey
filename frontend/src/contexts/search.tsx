import type { MessageSearch } from "sdk";
import { createContext, type ParentProps, useContext } from "solid-js";
import { createStore, type SetStoreFunction, type Store } from "solid-js/store";

export type SearchSort = "newest" | "oldest" | "relevancy";

export type SearchState = {
	query: string;
	results?: MessageSearch;
	loading: boolean;
	author?: string[];
	before?: string;
	after?: string;
	channel?: string[];
	sort?: SearchSort;
};

type SearchContextType = {
	states: Store<Record<string, SearchState>>;
	setStates: SetStoreFunction<Record<string, SearchState>>;
};

const SearchContext = createContext<SearchContextType>();

export const SearchProvider = (props: ParentProps) => {
	const [states, setStates] = createStore<Record<string, SearchState>>({});

	return (
		<SearchContext.Provider value={{ states, setStates }}>
			{props.children}
		</SearchContext.Provider>
	);
};

export const useSearch = () => {
	const ctx = useContext(SearchContext);
	if (!ctx) {
		throw new Error("useSearch must be used within a SearchProvider");
	}
	return ctx;
};
