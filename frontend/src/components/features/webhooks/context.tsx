import type { Webhook } from "sdk";
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

type WebhookCreate = {
	name: string;
	avatar?: string | null;
};

type WebhookUpdate = {
	name?: string | null;
	avatar?: string | null;
	channel_id?: string | null;
	rotate_token?: boolean;
};

export type WebhookScope =
	| { type: "room"; room_id: string }
	| { type: "channel"; channel_id: string };

export type WebhookDraft =
	| {
			state: "create";
			nonce: string;
			channel_id?: string;
			create: WebhookCreate;
	  }
	| { state: "update"; data: Webhook; update: WebhookUpdate }
	| { state: "clean"; data: Webhook }
	| { state: "delete"; data: Webhook };

export type WebhooksState = {
	drafts: WebhookDraft[];
	update: SetStoreFunction<WebhookDraft[]>;

	create(channel_id?: string): string;
	remove(id: string): void;
	restore(id: string): void;
	reset(): void;
	save(): Promise<void>;
	refetch(): Promise<void>;
	updateDraft(id: string, fields: Partial<WebhookUpdate>): void;

	readonly dirty: boolean;
	readonly saving: boolean;
};

const WebhooksContext = createContext<WebhooksState>();

export type WebhooksProviderProps = ParentProps<{
	scope: WebhookScope;
	defaultChannelId?: () => string | undefined;
}>;

export const WebhooksProvider = (props: WebhooksProviderProps) => {
	const api = useApi();
	const [drafts, update] = createStore<WebhookDraft[]>([]);
	const [saving, setSaving] = createSignal(false);

	const refetch = async () => {
		const { data } =
			props.scope.type === "room"
				? await api.client.http.GET("/api/v1/room/{room_id}/webhook", {
						params: { path: { room_id: props.scope.room_id } },
					})
				: await api.client.http.GET("/api/v1/channel/{channel_id}/webhook", {
						params: { path: { channel_id: props.scope.channel_id } },
					});
		const loaded: WebhookDraft[] = (data?.items ?? []).map((webhook) => ({
			state: "clean",
			data: webhook,
		}));
		update(loaded);
	};

	const save = async () => {
		setSaving(true);
		try {
			const operations: {
				draft: WebhookDraft;
				promise: Promise<{ error?: unknown }>;
			}[] = [];
			for (const draft of drafts) {
				if (draft.state === "create") {
					const channel_id =
						draft.channel_id ??
						(props.scope.type === "channel"
							? props.scope.channel_id
							: props.defaultChannelId?.());
					operations.push({
						draft: unwrap(draft),
						promise: channel_id
							? api.client.http.POST("/api/v1/channel/{channel_id}/webhook", {
									params: { path: { channel_id } },
									body: draft.create,
								})
							: api.client.http.POST("/api/v1/room/{room_id}/webhook", {
									params: {
										path: {
											room_id:
												props.scope.type === "room"
													? props.scope.room_id
													: "",
										},
									},
									body: draft.create,
								}),
					});
				} else if (draft.state === "update") {
					operations.push({
						draft: unwrap(draft),
						promise: api.client.http.PATCH("/api/v1/webhook/{webhook_id}", {
							params: { path: { webhook_id: draft.data.id } },
							body: draft.update,
						}),
					});
				} else if (draft.state === "delete") {
					operations.push({
						draft: unwrap(draft),
						promise: api.client.http.DELETE("/api/v1/webhook/{webhook_id}", {
							params: { path: { webhook_id: draft.data.id } },
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
						"failed to save webhook",
						operation.draft,
						result.reason,
					);
					return [operation.draft];
				}
				if (result.value.error) {
					console.error(
						"failed to save webhook",
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

	const create = (channel_id?: string) => {
		const nonce = uuidv7();
		update(drafts.length, {
			state: "create",
			nonce,
			channel_id:
				channel_id ??
				(props.scope.type === "channel"
					? props.scope.channel_id
					: props.defaultChannelId?.()),
			create: {
				name: "New Webhook",
				avatar: null,
			},
		});
		return nonce;
	};

	const remove = (id: string) => {
		const index = drafts.findIndex((draft) =>
			draft.state === "create" ? draft.nonce === id : draft.data.id === id,
		);
		if (index === -1) return;

		if (drafts[index].state === "create") {
			update((prev) => prev.filter((_, i) => i !== index));
		} else {
			update(index, { state: "delete" });
		}
	};

	const restore = (id: string) => {
		const index = drafts.findIndex((draft) =>
			draft.state === "create" ? draft.nonce === id : draft.data.id === id,
		);
		if (index === -1) return;
		const draft = drafts[index];
		if (draft.state === "create") return;
		update(index, { state: "clean", data: draft.data });
	};

	const reset = () => {
		update((prev) =>
			prev
				.filter((draft) => draft.state !== "create")
				.map((draft) => ({ state: "clean", data: draft.data })),
		);
	};

	const updateDraft = (id: string, fields: Partial<WebhookUpdate>) => {
		update(
			(draft) =>
				draft.state === "create" ? draft.nonce === id : draft.data.id === id,
			(draft) => {
				if (draft.state === "create") {
					const create = { ...draft.create };
					if (fields.name !== undefined && fields.name !== null) {
						create.name = fields.name;
					}
					if (fields.avatar !== undefined) {
						create.avatar = fields.avatar;
					}
					return {
						...draft,
						channel_id:
							fields.channel_id === undefined
								? draft.channel_id
								: (fields.channel_id ?? undefined),
						create,
					};
				}

				const currentData = draft.data;
				const finalUpdate: WebhookUpdate =
					draft.state === "update" ? { ...draft.update } : {};
				for (const [key, value] of Object.entries(fields)) {
					(finalUpdate as Record<string, unknown>)[key] = value;
				}

				let isClean = true;
				for (const [key, value] of Object.entries(finalUpdate)) {
					const origValue = (currentData as Record<string, unknown>)[key];
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

	const state: WebhooksState = {
		drafts,
		update,

		create,
		remove,
		restore,
		reset,
		save,
		refetch,
		updateDraft,

		get dirty() {
			return drafts.some((draft) => draft.state !== "clean");
		},

		get saving() {
			return saving();
		},
	};

	return (
		<WebhooksContext.Provider value={state}>
			{props.children}
		</WebhooksContext.Provider>
	);
};

export function useWebhooks() {
	const ctx = useContext(WebhooksContext);
	if (!ctx) throw new Error("useWebhooks must be called in a WebhooksProvider");
	return ctx;
}
