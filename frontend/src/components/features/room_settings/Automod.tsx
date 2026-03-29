import fuzzysort from "fuzzysort";
import type { Room, AutomodRule as SdkAutomodRule } from "sdk";
import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { createStore } from "solid-js/store";
import { uuidv7 } from "uuidv7";
import { useApi2, useChannels2 } from "@/api";
import { Savebar } from "@/atoms/Savebar.tsx";
import { usePermissions } from "@/hooks/usePermissions.ts";
import { useCurrentUser } from "../../../contexts/currentUser.tsx";
import { useModals } from "../../../contexts/modal";
import { AutomodRule } from "../automod_editor/AutomodRule.tsx";

// clean = not touched, data is straight from the server
// draft = not yet created
// edited = rule exists on server, has unsaved changes
export type RuleState = "clean" | "draft" | "edited";

export type UiAutomodRule = SdkAutomodRule & { state: RuleState };

export function Automod(props: VoidProps<{ room: Room }>) {
	const api2 = useApi2();
	const channels2 = useChannels2();
	const [, modalCtl] = useModals();
	const currentUser = useCurrentUser();

	const roomChannels = createMemo(() => {
		return [...channels2.cache.values()].filter(
			(c) => c.room_id === props.room.id,
		);
	});

	const [rules, { refetch }] = createResource(async () => {
		const { data } = await api2.client.http.GET(
			"/api/v1/room/{room_id}/automod/rule",
			{ params: { path: { room_id: props.room.id } } },
		);
		return (data ?? []) as UiAutomodRule[];
	});

	const removeRule = (rule_id: string) => () => {
		modalCtl.confirm("Are you sure you want to delete this rule?", (conf) => {
			if (!conf) return;
			api2.client.http
				.DELETE("/api/v1/room/{room_id}/automod/rule/{rule_id}", {
					params: { path: { room_id: props.room.id, rule_id } },
				})
				.then(() => {
					refetch();
				});
		});
	};

	const [search, setSearch] = createSignal("");
	const [draftRules, setDraftRules] = createStore<UiAutomodRule[]>([]);
	const [ruleStates, setRuleStates] = createSignal<Record<string, RuleState>>(
		{},
	);

	const filteredRules = () => {
		const query = search();
		const allRules = [...(rules() || []), ...draftRules];
		if (!query) return allRules;
		const results = fuzzysort.go(query, allRules, {
			key: "name",
			threshold: -10000,
		});
		return results.map((r) => r.obj);
	};

	const create = () => {
		const draftRule: UiAutomodRule = {
			id: uuidv7(),
			room_id: props.room.id,
			name: "New Rule",
			enabled: false,
			trigger: {
				type: "TextKeywords",
				keywords: [],
				allow: [],
			},
			actions: [],
			except_roles: [],
			except_channels: [],
			except_nsfw: false,
			include_everyone: false,
			target: "Content",
			state: "draft",
		};
		setDraftRules([...draftRules, draftRule]);
		setRuleStates({ ...ruleStates(), [draftRule.id]: "draft" });
	};

	createEffect(() => {
		// console.log("new draft rules", JSON.parse(JSON.stringify(draftRules)));
	});

	const updateRule = () => {
		// TODO
	};

	const setRuleState = (ruleId: string, state: RuleState) => {
		setRuleStates({ ...ruleStates(), [ruleId]: state });
	};

	const hasUnsavedChanges = () => {
		const states = ruleStates();
		return Object.values(states).some((s) => s === "draft" || s === "edited");
	};

	const user_id = () => currentUser()?.id;
	const perms = usePermissions(
		user_id,
		() => props.room.id,
		() => undefined,
	);

	const handleSave = async () => {
		const states = ruleStates();
		const draftRulesToSave = draftRules;

		for (const [ruleId, state] of Object.entries(states)) {
			if (state === "draft") {
				const rule = draftRulesToSave.find((r) => r.id === ruleId);
				if (rule) {
					// Clean arrays before saving
					const cleanedRule = {
						...rule,
						trigger: {
							...rule.trigger,
							keywords: (rule.trigger as any).keywords?.filter(
								(k: string) => k.trim() !== "",
							),
							deny: (rule.trigger as any).deny?.filter(
								(d: string) => d.trim() !== "",
							),
							allow: (rule.trigger as any).allow?.filter(
								(a: string) => a.trim() !== "",
							),
							hostnames: (rule.trigger as any).hostnames?.filter(
								(h: string) => h.trim() !== "",
							),
						},
					};
					await api2.client.http.POST("/api/v1/room/{room_id}/automod/rule", {
						params: { path: { room_id: props.room.id } },
						body: cleanedRule,
					});
				}
			} else if (state === "edited") {
				const rule = filteredRules().find((r) => r.id === ruleId);
				if (rule) {
					await api2.client.http.PATCH(
						"/api/v1/room/{room_id}/automod/rule/{rule_id}",
						{
							params: { path: { room_id: props.room.id, rule_id: ruleId } },
							body: {
								name: rule.name,
								enabled: rule.enabled,
							},
						},
					);
				}
			}
		}

		setRuleStates({});
		setDraftRules([]);
		await refetch();
	};

	return (
		<div class="room-settings-automod">
			<h2>automod</h2>
			<Show when={perms.has("RoomManage")}>
				<header class="applications-header">
					<input
						type="search"
						placeholder="search"
						aria-label="search"
						onInput={(e) => setSearch(e.target.value)}
					/>
					<button type="button" class="primary big" onClick={create}>
						create
					</button>
				</header>
			</Show>
			<Show when={filteredRules()?.length} fallback="no rules">
				<ul class="automod-rules-list">
					<For each={filteredRules()} fallback="no items">
						{(rule) => (
							<AutomodRule
								rule={rule}
								ruleStates={ruleStates()}
								setRuleState={setRuleState}
								draftRules={draftRules}
								setDraftRules={setDraftRules}
								onUpdate={updateRule}
								onDelete={removeRule(rule.id)}
								room_id={props.room.id}
								channels={roomChannels}
							/>
						)}
					</For>
				</ul>
			</Show>
			<Savebar
				onSave={handleSave}
				onCancel={() => {
					setDraftRules([]);
					setRuleStates({});
					refetch();
				}}
				show={hasUnsavedChanges()}
			/>
		</div>
	);
}
