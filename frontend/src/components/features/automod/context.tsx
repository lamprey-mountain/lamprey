import {
	createContext,
	createEffect,
	createResource,
	type ParentProps,
	useContext,
} from "solid-js";
import { createStore, type SetStoreFunction } from "solid-js/store";
import type { AutomodRule, AutomodRuleCreate, AutomodRuleUpdate } from "ts-sdk";
import { uuidv7 } from "uuidv7";
import { useApi } from "@/api";

export type AutomodRuleDraft =
	| { state: "create"; nonce: string; create: AutomodRuleCreate }
	| { state: "update"; rule: AutomodRule; update: AutomodRuleUpdate }
	| { state: "clean"; rule: AutomodRule }
	| { state: "delete"; rule: AutomodRule };

export type AutomodState = {
	rules: AutomodRuleDraft[];
	update: SetStoreFunction<AutomodRuleDraft[]>;
	updateRule: (draft: AutomodRuleDraft, key: string, value: any) => void;
	updateAction: (
		draft: AutomodRuleDraft,
		index: number,
		key: string,
		value: any,
	) => void;
	removeAction: (draft: AutomodRuleDraft, index: number) => void;

	create(): void;
	remove(id: string): void;
	save(): Promise<void>;
	reset(): void;
	refetch(): Promise<void>;

	readonly roomId: string;

	/** whether any changes have been made to these automod rules */
	readonly dirty: boolean;
};

const AutomodContext = createContext<AutomodState>();

export type AutomodProviderProps = ParentProps<{ room_id: string }>;

export const AutomodProvider = (props: AutomodProviderProps) => {
	const api = useApi();
	const [rules, update] = createStore<AutomodRuleDraft[]>([]);

	// TODO: maybe use this instead?
	// const [source] = createResource(() => props.room_id, async () => {
	// 	const { data } = await api.client.http.GET(
	// 		"/api/v1/room/{room_id}/automod/rule",
	// 		{ params: { path: { room_id: props.room_id } } },
	// 	);
	// 	return data;
	// })

	const refetch = async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/room/{room_id}/automod/rule",
			{ params: { path: { room_id: props.room_id } } },
		);
		// TODO: error handling
		const loadedRules: AutomodRuleDraft[] = (data ?? []).map((rule) => ({
			state: "clean",
			rule,
		}));
		update(loadedRules);
	};

	const save = async () => {
		for (const r of rules) {
			if (r.state === "create") {
				await api.client.http.POST("/api/v1/room/{room_id}/automod/rule", {
					params: { path: { room_id: props.room_id } },
					body: r.create,
				});
			} else if (r.state === "update") {
				await api.client.http.PATCH(
					"/api/v1/room/{room_id}/automod/rule/{rule_id}",
					{
						params: { path: { room_id: props.room_id, rule_id: r.rule.id } },
						body: r.update,
					},
				);
			} else if (r.state === "delete") {
				await api.client.http.DELETE(
					"/api/v1/room/{room_id}/automod/rule/{rule_id}",
					{
						params: { path: { room_id: props.room_id, rule_id: r.rule.id } },
					},
				);
			}
		}
		await refetch();
	};

	const create = () => {
		update(rules.length, {
			state: "create",
			nonce: uuidv7(),
			create: {
				name: "New Rule",
				enabled: true,
				trigger: { type: "TextKeywords", keywords: [], allow: [] },
				actions: [],
				except_roles: [],
				except_channels: [],
				except_nsfw: false,
				include_everyone: false,
				target: "Content",
			},
		});
	};

	const remove = (id: string) => {
		const index = rules.findIndex((r) =>
			r.state === "create" ? r.nonce === id : r.rule.id === id,
		);
		if (index === -1) return;

		const draft = rules[index];
		if (draft.state === "create") {
			update((prev) => {
				const before = prev.slice(0, index);
				const after = prev.slice(index + 1);
				return [...before, ...after];
			});
		} else {
			update(index, { state: "delete" });
		}
	};

	const reset = () => {
		update((prev) => {
			return prev
				.filter((i) => i.state !== "create")
				.map((draft) => ({ state: "clean", rule: draft.rule }));
		});
	};

	const updateRule = (draft: AutomodRuleDraft, key: string, value: any) => {
		const index = rules.findIndex((r) =>
			r.state === "create" && draft.state === "create"
				? r.nonce === draft.nonce
				: r.state !== "create" && draft.state !== "create"
					? r.rule.id === draft.rule.id
					: false,
		);
		if (index === -1) return;

		const targetDraft = rules[index];
		if (targetDraft.state === "create") {
			update(index, "create", key, value);
		} else if (targetDraft.state === "clean") {
			update(index, {
				state: "update",
				update: { [key]: value },
			});
		} else if (targetDraft.state === "update") {
			update(index, "update", key, value);
		}
	};

	const updateAction = (
		draft: AutomodRuleDraft,
		actionIndex: number,
		key: string,
		value: any,
	) => {
		const ruleIndex = rules.findIndex((r) =>
			r.state === "create" && draft.state === "create"
				? r.nonce === draft.nonce
				: r.state !== "create" && draft.state !== "create"
					? r.rule.id === draft.rule.id
					: false,
		);
		if (ruleIndex === -1) return;

		const targetDraft = rules[ruleIndex];
		if (targetDraft.state === "create") {
			update(ruleIndex, "create", "actions", actionIndex, key, value);
		} else if (targetDraft.state === "clean") {
			const newActions = [...targetDraft.rule.actions];
			newActions[actionIndex] = { ...newActions[actionIndex], [key]: value };
			update(ruleIndex, {
				state: "update",
				update: { actions: newActions },
			});
		} else if (targetDraft.state === "update") {
			if (!targetDraft.update.actions) {
				const newActions = [...targetDraft.rule.actions];
				newActions[actionIndex] = { ...newActions[actionIndex], [key]: value };
				update(ruleIndex, "update", "actions", newActions);
			} else {
				update(ruleIndex, "update", "actions", actionIndex, key, value);
			}
		}
	};

	const removeAction = (draft: AutomodRuleDraft, actionIndex: number) => {
		const ruleIndex = rules.findIndex((r) =>
			r.state === "create" && draft.state === "create"
				? r.nonce === draft.nonce
				: r.state !== "create" && draft.state !== "create"
					? r.rule.id === draft.rule.id
					: false,
		);
		if (ruleIndex === -1) return;

		const targetDraft = rules[ruleIndex];
		const currentActions =
			targetDraft.state === "create"
				? targetDraft.create.actions
				: targetDraft.state === "update"
					? (targetDraft.update.actions ?? targetDraft.rule.actions)
					: targetDraft.rule.actions;
		const newActions = currentActions.filter((_, i) => i !== actionIndex);

		if (targetDraft.state === "create") {
			update(ruleIndex, "create", "actions", newActions);
		} else if (targetDraft.state === "clean") {
			update(ruleIndex, {
				state: "update",
				update: { actions: newActions },
			});
		} else if (targetDraft.state === "update") {
			update(ruleIndex, "update", "actions", newActions);
		}
	};

	createEffect(() => {
		refetch();
	});

	const state: AutomodState = {
		rules,
		update,
		updateRule,
		updateAction,
		removeAction,

		create,
		remove,
		save,
		reset,
		refetch,

		get roomId() {
			return props.room_id;
		},

		get dirty() {
			return rules.some((r) => r.state !== "clean");
		},
	};

	return (
		<AutomodContext.Provider value={state}>
			{props.children}
		</AutomodContext.Provider>
	);
};

export const useAutomod = () => {
	const ctx = useContext(AutomodContext);
	if (!ctx) throw new Error("useAutomod must be called in an AutomodProvider");
	return ctx;
};
