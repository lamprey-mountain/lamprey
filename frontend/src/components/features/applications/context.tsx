import type { Application, ApplicationCreate, ApplicationUpdate } from "sdk";
import {
	createContext,
	createEffect,
	type ParentProps,
	useContext,
} from "solid-js";
import { createStore, type SetStoreFunction } from "solid-js/store";
import { uuidv7 } from "uuidv7";
import { useApi } from "@/api";
import { deepEqual } from "@/utils/deepEqual";

export type ApplicationDraft =
	| { state: "create"; nonce: string; create: ApplicationCreate }
	| { state: "update"; data: Application; update: ApplicationUpdate }
	| { state: "clean"; data: Application }
	| { state: "delete"; data: Application };

export type ApplicationsState = {
	apps: ApplicationDraft[];
	update: SetStoreFunction<ApplicationDraft[]>;

	create(): void;
	reset: () => void;
	save: () => Promise<void>;
	refetch: () => Promise<void>;
	updateDraft: (id: string, newFields: Partial<ApplicationUpdate>) => void;

	/** whether any changes have been made to these applications */
	readonly dirty: boolean;
};

const ApplicationsContext = createContext<ApplicationsState>();

export const ApplicationsProvider = (props: ParentProps) => {
	const api = useApi();
	const [apps, update] = createStore<ApplicationDraft[]>([]);

	const refetch = async () => {
		const { data } = await api.client.http.GET("/api/v1/app", {
			params: { query: { limit: 100 } },
		});
		const loadedApps: ApplicationDraft[] = (data?.items ?? []).map((app) => ({
			state: "clean",
			data: app,
		}));
		update(loadedApps);
	};

	const save = async () => {
		const proms = [];
		for (const a of apps) {
			if (a.state === "create") {
				proms.push(
					api.client.http.POST("/api/v1/app", {
						body: a.create,
					}),
				);
			} else if (a.state === "update") {
				proms.push(
					api.client.http.PATCH("/api/v1/app/{app_id}", {
						params: { path: { app_id: a.data.id } },
						body: a.update,
					}),
				);
			} else if (a.state === "delete") {
				proms.push(
					api.client.http.DELETE("/api/v1/app/{app_id}", {
						params: { path: { app_id: a.data.id } },
					}),
				);
			}
		}
		await Promise.allSettled(proms);
		await refetch();
	};

	const create = () => {
		update(apps.length, {
			state: "create",
			nonce: uuidv7(),
			create: {
				name: "New Application",
				bridge: null,
				public: false,
			},
		});
	};

	const reset = () => {
		update((prev) => {
			return prev
				.filter((i) => i.state !== "create")
				.map((draft) => ({ state: "clean", data: draft.data }));
		});
	};

	const updateDraft = (id: string, newFields: Partial<ApplicationUpdate>) => {
		update(
			(a) => (a.state === "create" ? a.nonce === id : a.data.id === id),
			(draft) => {
				if (draft.state === "create") {
					return { ...draft, create: { ...draft.create, ...newFields } };
				}

				const currentData =
					draft.state === "clean" || draft.state === "delete"
						? draft.data
						: draft.data;

				const finalUpdate: Partial<ApplicationUpdate> =
					draft.state === "update" ? { ...draft.update } : {};
				for (const [key, value] of Object.entries(newFields)) {
					(finalUpdate as any)[key] = value;
				}

				let isClean = true;
				for (const [key, value] of Object.entries(finalUpdate)) {
					const origValue = (currentData as any)[key];
					if (value !== undefined && !deepEqual(value, origValue)) {
						isClean = false;
						break;
					}
				}

				if (isClean) {
					return { state: "clean", data: currentData };
				}
				return { state: "update", data: currentData, update: finalUpdate };
			},
		);
	};

	createEffect(() => {
		refetch();
	});

	const dirty = () => apps.some((a) => a.state !== "clean");

	const state: ApplicationsState = {
		apps,
		update,

		create,
		reset,
		save,
		refetch,
		updateDraft,

		get dirty() {
			return dirty();
		},
	};

	return (
		<ApplicationsContext.Provider value={state}>
			{props.children}
		</ApplicationsContext.Provider>
	);
};

export function useApplications() {
	const ctx = useContext(ApplicationsContext);
	if (!ctx)
		throw new Error(
			"useApplications must be called in an ApplicationsProvider",
		);
	return ctx;
}
