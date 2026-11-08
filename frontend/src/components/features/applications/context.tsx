import type { Application, ApplicationCreate, ApplicationPatch } from "sdk";
import {
	createContext,
	createEffect,
	createSignal,
	type ParentProps,
	useContext,
} from "solid-js";
import { createStore, type SetStoreFunction, unwrap } from "solid-js/store";
import { uuidv7 } from "uuidv7";
import { useApi } from "@/api";
import { deepEqual } from "@/utils/deepEqual";

export type ApplicationDraft =
	| { state: "create"; nonce: string; create: ApplicationCreate }
	| { state: "update"; data: Application; update: ApplicationPatch }
	| { state: "clean"; data: Application }
	| { state: "delete"; data: Application };

export type ApplicationsState = {
	apps: ApplicationDraft[];
	update: SetStoreFunction<ApplicationDraft[]>;

	create(): void;
	reset: () => void;
	save: () => Promise<void>;
	refetch: () => Promise<void>;
	updateDraft: (id: string, newFields: Partial<ApplicationPatch>) => void;

	/** whether any changes have been made to these applications */
	readonly dirty: boolean;

	/** whether the current changes are being saved */
	readonly saving: boolean;
};

const ApplicationsContext = createContext<ApplicationsState>();

export const ApplicationsProvider = (props: ParentProps) => {
	const api = useApi();
	const [apps, update] = createStore<ApplicationDraft[]>([]);
	const [saving, setSaving] = createSignal(false);

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
		setSaving(true);
		try {
			const operations: {
				draft: ApplicationDraft;
				promise: Promise<{ error?: unknown }>;
			}[] = [];
			for (const a of apps) {
				if (a.state === "create") {
					operations.push({
						draft: unwrap(a),
						promise: api.client.http.POST("/api/v1/app", {
							body: a.create,
						}),
					});
				} else if (a.state === "update") {
					operations.push({
						draft: unwrap(a),
						promise: api.client.http.PATCH("/api/v1/app/{app_id}", {
							params: { path: { app_id: a.data.id } },
							body: a.update,
						}),
					});
				} else if (a.state === "delete") {
					operations.push({
						draft: unwrap(a),
						promise: api.client.http.DELETE("/api/v1/app/{app_id}", {
							params: { path: { app_id: a.data.id } },
						}),
					});
				}
			}

			const results = await Promise.allSettled(
				operations.map((operation) => operation.promise),
			);
			const failed = results.flatMap((result, i) => {
				const operation = operations[i];
				if (result.status === "rejected") {
					console.error(
						"failed to save application",
						operation.draft,
						result.reason,
					);
					return [operation.draft];
				}
				if (result.value.error) {
					console.error(
						"failed to save application",
						operation.draft,
						result.value.error,
					);
					return [operation.draft];
				}
				return [];
			});

			await refetch();
			if (failed.length > 0) {
				const failedIds = new Set(
					failed.map((draft) =>
						draft.state === "create" ? draft.nonce : draft.data.id,
					),
				);
				update((prev) => [
					...prev.filter((draft) => {
						const id = draft.state === "create" ? draft.nonce : draft.data.id;
						return !failedIds.has(id);
					}),
					...failed,
				]);
			}
		} finally {
			setSaving(false);
		}
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

	const updateDraft = (id: string, newFields: Partial<ApplicationPatch>) => {
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

				const finalUpdate: Partial<ApplicationPatch> =
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

		get saving() {
			return saving();
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
