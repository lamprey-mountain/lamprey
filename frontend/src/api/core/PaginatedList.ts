import { createStore, reconcile } from "solid-js/store";

export type ListState = {
	ids: string[];
	has_more: boolean;
	cursor?: string;
	isLoading: boolean;
	error?: unknown;
};

export class PaginatedList {
	public state: ListState;
	private setState: (setter: any) => void;

	constructor(initialState?: Partial<ListState>) {
		const [state, setState] = createStore<ListState>({
			ids: [],
			has_more: true,
			cursor: undefined,
			isLoading: false,
			...initialState,
		} as ListState);
		this.state = state;
		this.setState = setState;
	}

	// Used when fetching the next page from the API
	appendPage(newIds: string[], has_more: boolean, cursor?: string) {
		this.setState((prev: ListState) => ({
			ids: [...prev.ids, ...newIds],
			has_more,
			cursor,
			isLoading: false,
		}));
	}

	// Used for WebSocket CREATE events
	prependId(id: string) {
		if (!this.state.ids.includes(id)) {
			this.setState((prev: ListState) => ({
				...prev,
				ids: [id, ...prev.ids],
			}));
		}
	}

	// Used for WebSocket CREATE events (if appending to bottom)
	appendId(id: string) {
		if (!this.state.ids.includes(id)) {
			this.setState((prev: ListState) => ({
				...prev,
				ids: [...prev.ids, id],
			}));
		}
	}

	// Used for WebSocket DELETE Events
	removeId(id: string) {
		this.setState((prev: ListState) => ({
			...prev,
			ids: prev.ids.filter((i) => i !== id),
		}));
	}

	setLoading(isLoading: boolean) {
		this.setState({ isLoading });
	}

	setError(error: unknown) {
		this.setState({ error, isLoading: false });
	}
}
