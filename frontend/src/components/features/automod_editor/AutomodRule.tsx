import type {
	AutomodAction,
	AutomodTarget,
	AutomodTrigger,
	Channel,
} from "sdk";
import { createEffect, createSignal, Show } from "solid-js";
import type { SetStoreFunction } from "solid-js/store";
import type { RuleState } from "../room_settings/Automod.tsx";
import { ActionsEditor } from "./ActionsEditor.tsx";
import { TriggerEditor } from "./TriggerEditor.tsx";

export interface UiAutomodRule {
	id: string;
	room_id: string;
	name: string;
	enabled: boolean;
	trigger: AutomodTrigger;
	actions: AutomodAction[];
	except_roles: string[];
	except_channels: string[];
	except_nsfw: boolean;
	include_everyone: boolean;
	target: AutomodTarget;
	state: RuleState;
}

export function AutomodRule(props: {
	rule: UiAutomodRule;
	ruleStates: Record<string, RuleState>;
	setRuleState: (ruleId: string, state: RuleState) => void;
	draftRules: UiAutomodRule[];
	setDraftRules: SetStoreFunction<UiAutomodRule[]>;
	onUpdate: (ruleId: string, data: { name: string; enabled: boolean }) => void;
	onDelete: (ruleId: string) => void;
	room_id: string;
	channels: () => Channel[];
}) {
	const [name, setName] = createSignal(props.rule.name);
	const [enabled, setEnabled] = createSignal(props.rule.enabled);
	const [isEditing, setIsEditing] = createSignal(false);
	let inputEl: HTMLInputElement | undefined;

	const ruleState = () => props.ruleStates[props.rule.id] ?? "clean";

	createEffect(() => {
		setName(props.rule.name);
		setEnabled(props.rule.enabled);
	});

	const handleNameChange = (newName: string) => {
		setName(newName);
		if (props.rule.state === "draft") {
			props.setRuleState(props.rule.id, "draft");
			props.setDraftRules((r) => r.id === props.rule.id, "name", newName);
		} else {
			props.setRuleState(props.rule.id, "edited");
		}
	};

	const handleBlur = () => {
		setIsEditing(false);
	};

	const handleClick = () => {
		setIsEditing(true);
		setTimeout(() => {
			inputEl?.focus();
			inputEl?.select();
		}, 0);
	};

	const handleKeyDown = (e: KeyboardEvent) => {
		if (e.key === "Enter") {
			handleBlur();
		}
	};

	const updateNested = (path: string, value: any) => {
		props.setRuleState(
			props.rule.id,
			props.rule.state === "draft" ? "draft" : "edited",
		);

		if (props.rule.state === "draft") {
			// @ts-expect-error - dynamic path update
			props.setDraftRules((r) => r.id === props.rule.id, path, value);
		} else {
			// If it's a "clean" or "edited" server rule, you might need a local
			// "modifiedRules" store similar to draftRules to track changes
			// before hitting the PATCH endpoint.
		}
	};

	const updateTriggerValue = (key: string, val: any) => {
		props.setDraftRules(
			(r) => r.id === props.rule.id,
			"trigger" as any,
			key as any,
			val,
		);
	};

	const updateTriggerType = (type: AutomodTrigger["type"]) => {
		// Define the default structure for each trigger type
		const defaults: Record<string, any> = {
			TextKeywords: { type: "TextKeywords", keywords: [], allow: [] },
			TextRegex: { type: "TextRegex", deny: [], allow: [] },
			TextLinks: { type: "TextLinks", hostnames: [], whitelist: false },
			TextBuiltin: { type: "TextBuiltin", list: "Profanity" },
			MediaScan: { type: "MediaScan", scanner: "Nsfw" },
		};

		props.setDraftRules(
			(r) => r.id === props.rule.id,
			"trigger",
			defaults[type], // This replaces the whole trigger object ONLY when switching types
		);
	};

	const updateActionValue = (idx: number, key: string, val: any) => {
		props.setDraftRules(
			(r) => r.id === props.rule.id,
			"actions" as any,
			idx,
			key as any,
			val,
		);
	};

	const updateActionType = (idx: number, type: AutomodAction["type"]) => {
		const defaults: Record<string, any> = {
			Block: { type: "Block", message: "" },
			Timeout: { type: "Timeout", duration: 3600000 },
			Remove: { type: "Remove" },
			SendAlert: { type: "SendAlert", channel_id: "" },
		};
		props.setDraftRules(
			(r) => r.id === props.rule.id,
			"actions",
			idx,
			defaults[type],
		);
	};

	const addAction = () => {
		// Default to 'Block' for Content or 'SendAlert' for Member
		const newAction =
			props.rule.target === "Member"
				? { type: "SendAlert", channel_id: "" }
				: { type: "Block", message: "" };

		props.setDraftRules(
			(r) => r.id === props.rule.id,
			"actions" as any,
			(actions) => [...(actions || []), newAction as any],
		);
	};

	const removeAction = (idx: number) => {
		props.setDraftRules(
			(r) => r.id === props.rule.id,
			"actions",
			(actions) => actions.filter((_, i) => i !== idx),
		);
	};

	return (
		<li
			class="automod-rule"
			classList={{
				"draft-rule": ruleState() === "draft",
				"edited-rule": ruleState() === "edited",
			}}
		>
			<Show
				when={isEditing()}
				fallback={
					<h3 onClick={handleClick} style={{ cursor: "pointer" }}>
						{name()}
					</h3>
				}
			>
				<input
					ref={inputEl}
					type="text"
					value={name()}
					onInput={(e) => handleNameChange(e.target.value)}
					onBlur={handleBlur}
					onKeyDown={handleKeyDown}
				/>
			</Show>

			<TriggerEditor
				trigger={props.rule.trigger}
				updateTriggerValue={updateTriggerValue}
				updateTriggerType={updateTriggerType}
			/>

			<ActionsEditor
				target={props.rule.target}
				actions={props.rule.actions}
				updateActionValue={updateActionValue}
				updateActionType={updateActionType}
				addAction={addAction}
				removeAction={removeAction}
				room_id={props.room_id}
				channels={props.channels}
			/>
		</li>
	);
}
