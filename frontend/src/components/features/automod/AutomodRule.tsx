import { createSignal, For, Match, ParentProps, Show, Switch } from "solid-js";
import { Dynamic } from "solid-js/web";
import type { AutomodAction, AutomodRule, AutomodTrigger } from "ts-sdk";
import { useApi, useChannels, useRoles } from "@/api";
import { CheckboxOptionWithLabel } from "@/atoms/CheckboxOption";
import { Dropdown, MultiDropdown } from "@/atoms/Dropdown";
import { Editable } from "@/atoms/Editable";
import {
	ActionBlock,
	ActionRemove,
	ActionSendAlert,
	ActionTimeout,
} from "./Action";
import { type AutomodRuleDraft, useAutomod } from "./context";
import {
	TriggerBuiltin,
	TriggerKeywords,
	TriggerLinks,
	TriggerMediaScan,
	TriggerRegex,
} from "./Trigger";

export type AutomodRuleProps = {
	draft: AutomodRuleDraft;
	open?: boolean;
};

// TODO: dropdown icons for trigger types
// TODO: dropdown icons for action types
// TODO: collapseable rules (like details/summary)

export const AutomodRuleEditor = (props: AutomodRuleProps) => {
	const am = useAutomod();
	const api = useApi();

	const open = () => (props.open ?? true) && props.draft.state !== "delete";

	// TODO: refactor/deduplicate these functions?

	const name = () => {
		const d = props.draft;
		switch (d.state) {
			case "create":
				return d.create.name;
			case "update":
				return d.update.name ?? d.rule.name;
			case "clean":
				return d.rule.name;
			case "delete":
				return d.rule.name;
		}
	};

	const trigger = () => {
		const d = props.draft;
		switch (d.state) {
			case "create":
				return d.create.trigger;
			case "update":
				return d.update.trigger ?? d.rule.trigger;
			case "clean":
				return d.rule.trigger;
			case "delete":
				return d.rule.trigger;
		}
	};

	const actions = () => {
		const d = props.draft;
		switch (d.state) {
			case "create":
				return d.create.actions;
			case "update":
				return d.update.actions ?? d.rule.actions;
			case "clean":
				return d.rule.actions;
			case "delete":
				return d.rule.actions;
		}
	};

	const target = () => {
		const d = props.draft;
		switch (d.state) {
			case "create":
				return d.create.target;
			case "update":
				return d.update.target ?? d.rule.target;
			case "clean":
				return d.rule.target;
			case "delete":
				return d.rule.target;
		}
	};

	function matchesTrigger<T extends AutomodTrigger["type"]>(
		ty: T,
	): (AutomodTrigger & { type: T }) | false {
		const t = trigger();
		if (t.type === ty) {
			return t as AutomodTrigger & { type: T };
		} else {
			return false;
		}
	}

	const enabled = () => {
		const d = props.draft;
		switch (d.state) {
			case "create":
				return d.create.enabled;
			case "update":
				return d.update.enabled ?? d.rule.enabled;
			case "clean":
				return d.rule.enabled;
			case "delete":
				return d.rule.enabled;
		}
	};

	// TODO: fix typescript types
	const triggerDefaults: Record<string, any> = {
		TextKeywords: { type: "TextKeywords", keywords: [], allow: [] },
		TextRegex: { type: "TextRegex", deny: [], allow: [] },
		TextLinks: { type: "TextLinks", hostnames: [], whitelist: false },
		TextBuiltin: { type: "TextBuiltin", list: "Profanity" },
		MediaScan: { type: "MediaScan", scanner: "Nsfw" },
	};

	const actionDefaults: Record<string, any> = {
		Block: { type: "Block", message: null },
		Timeout: { type: "Timeout", duration: 5 * 60 * 1000 },
		Remove: { type: "Remove" },
		// TODO: default to an existing channel id?
		SendAlert: { type: "SendAlert", channel_id: "" },
	};

	const ruleId = () => {
		const d = props.draft;
		return d.state === "create" ? d.nonce : d.rule.id;
	};

	const roomRoles = () => api.roles.listByRoom(am.roomId);

	const exceptChannels = () => {
		const d = props.draft;
		switch (d.state) {
			case "create":
				return d.create.except_channels ?? [];
			case "update":
				return d.update.except_channels ?? d.rule.except_channels;
			case "clean":
				return d.rule.except_channels;
			case "delete":
				return d.rule.except_channels;
		}
	};

	const exceptRoles = () => {
		const d = props.draft;
		switch (d.state) {
			case "create":
				return d.create.except_roles ?? [];
			case "update":
				return d.update.except_roles ?? d.rule.except_roles;
			case "clean":
				return d.rule.except_roles;
			case "delete":
				return d.rule.except_roles;
		}
	};

	const exceptNsfw = () => {
		const d = props.draft;
		switch (d.state) {
			case "create":
				return d.create.except_nsfw ?? false;
			case "update":
				return d.update.except_nsfw ?? d.rule.except_nsfw;
			case "clean":
				return d.rule.except_nsfw;
			case "delete":
				return d.rule.except_nsfw;
		}
	};

	const includeEveryone = () => {
		const d = props.draft;
		switch (d.state) {
			case "create":
				return d.create.include_everyone ?? false;
			case "update":
				return d.update.include_everyone ?? d.rule.include_everyone;
			case "clean":
				return d.rule.include_everyone;
			case "delete":
				return d.rule.include_everyone;
		}
	};

	return (
		<div class="automod-rule" data-draft-state={props.draft.state}>
			<div class="header">
				<Show
					when={props.draft.state !== "delete"}
					fallback={<h2 class="name">{name()}</h2>}
				>
					<Editable
						wrapper="h2"
						value={name()}
						onSave={(name) => {
							am.updateRule(props.draft, "name", name);
						}}
						blur="save"
						class="name"
						autoselect
					/>

					<CheckboxOptionWithLabel
						id={`enabled-${ruleId()}`}
						seed={`enabled-${ruleId()}`}
						checked={enabled()}
						label="Enabled"
						onChange={(checked) =>
							am.updateRule(props.draft, "enabled", checked)
						}
					/>
					<button class="button danger" onClick={() => am.remove(ruleId())}>
						Delete
					</button>
				</Show>
			</div>
			<Show when={open()}>
				<div class="trigger">
					<h3>Trigger</h3>
					<label>
						<h3 class="dim">Trigger Type</h3>
						<Dropdown
							options={[
								{ item: "TextKeywords", label: "Text keywords" },
								{ item: "TextRegex", label: "Text regex" },
								{ item: "TextLinks", label: "Text links" },
								{ item: "TextBuiltin", label: "Builtin list" },
								{ item: "MediaScan", label: "Media scanner" },
							]}
							onSelect={(item) => {
								const newTrigger = structuredClone(triggerDefaults[item]);
								am.updateRule(props.draft, "trigger", newTrigger);
							}}
							selected={trigger().type}
						/>
					</label>
					<Switch>
						<Match when={matchesTrigger("TextKeywords")}>
							{(trigger) => (
								<TriggerKeywords draft={props.draft} trigger={trigger()} />
							)}
						</Match>
						<Match when={matchesTrigger("TextRegex")}>
							{(trigger) => (
								<TriggerRegex draft={props.draft} trigger={trigger()} />
							)}
						</Match>
						<Match when={matchesTrigger("TextLinks")}>
							{(trigger) => (
								<TriggerLinks draft={props.draft} trigger={trigger()} />
							)}
						</Match>
						<Match when={matchesTrigger("TextBuiltin")}>
							{(trigger) => (
								<TriggerBuiltin draft={props.draft} trigger={trigger()} />
							)}
						</Match>
						<Match when={matchesTrigger("MediaScan")}>
							{(trigger) => (
								<TriggerMediaScan draft={props.draft} trigger={trigger()} />
							)}
						</Match>
					</Switch>
				</div>
				<div class="actions">
					<h3>Actions</h3>
					<For each={actions()}>
						{(action, index) => {
							// only certain actions are allowed for certain targets
							function isActionAllowed(type: AutomodAction["type"]) {
								switch (target()) {
									case "Content":
										return true;
									case "Member":
										return type === "Block" || type === "SendAlert";
								}
							}

							function matchesAction<T extends AutomodAction["type"]>(
								ty: T,
							): (AutomodAction & { type: T }) | false {
								if (action.type === ty && isActionAllowed(ty)) {
									return action as AutomodAction & { type: T };
								} else {
									return false;
								}
							}

							return (
								<div class="action">
									<div class="top">
										<Dropdown
											// TODO: make typescript happy
											// TODO: move labels to i18n
											options={[
												{ item: "Block", label: "Block Message" },
												{ item: "Timeout", label: "Timeout Sender" },
												{ item: "Remove", label: "Remove Message" },
												{ item: "SendAlert", label: "Send Alert" },
											].filter((i) => isActionAllowed(i.item))}
											selected={action.type}
											onSelect={(type) => {
												const currentActions = actions();
												const newActions = [...currentActions];
												newActions[index()] = structuredClone(
													actionDefaults[type],
												);
												am.updateRule(props.draft, "actions", newActions);
											}}
										/>
										<button
											class="button link danger"
											onClick={() => {
												am.removeAction(props.draft, index());
											}}
										>
											remove
										</button>
									</div>
									<Switch>
										<Match when={matchesAction("Block")}>
											{(action) => (
												<ActionBlock
													draft={props.draft}
													index={index()}
													action={action()}
												/>
											)}
										</Match>
										<Match when={matchesAction("Timeout")}>
											{(action) => (
												<ActionTimeout
													draft={props.draft}
													index={index()}
													action={action()}
												/>
											)}
										</Match>
										<Match when={matchesAction("Remove")}>
											<ActionRemove />
										</Match>
										<Match when={matchesAction("SendAlert")}>
											{(action) => (
												<ActionSendAlert
													draft={props.draft}
													index={index()}
													action={action()}
												/>
											)}
										</Match>
									</Switch>
								</div>
							);
						}}
					</For>
					<button
						class="action create"
						onClick={() => {
							const currentActions = actions();
							const newActions = [
								...currentActions,
								structuredClone(actionDefaults.Block),
							];
							am.updateRule(props.draft, "actions", newActions);
						}}
					>
						+ create
					</button>
				</div>
				<div class="exceptions">
					<h3>Exceptions</h3>
					<label style="display: block; margin: 8px 0">
						<h3 class="dim">Exempt channels</h3>
						<MultiDropdown
							selected={exceptChannels()}
							onSelect={(id) =>
								am.updateRule(props.draft, "except_channels", [
									...exceptChannels(),
									id,
								])
							}
							onRemove={(id) =>
								am.updateRule(
									props.draft,
									"except_channels",
									exceptChannels().filter((c) => c !== id),
								)
							}
							options={[...api.channels.cache.values()]
								.filter((c) => c.type === "Text")
								.map((c) => ({ item: c.id, label: c.name }))}
							placeholder="Select channels..."
						/>
					</label>
					<label style="display: block; margin: 8px 0">
						<h3 class="dim">Exempt roles</h3>
						<MultiDropdown
							selected={exceptRoles()}
							onSelect={(id) =>
								am.updateRule(props.draft, "except_roles", [
									...exceptRoles(),
									id,
								])
							}
							onRemove={(id) =>
								am.updateRule(
									props.draft,
									"except_roles",
									exceptRoles().filter((r) => r !== id),
								)
							}
							options={roomRoles().map((r) => ({ item: r.id, label: r.name }))}
							placeholder="Select roles..."
						/>
					</label>
					<CheckboxOptionWithLabel
						id={`nsfw-${ruleId()}`}
						seed={`nsfw-${ruleId()}`}
						checked={exceptNsfw()}
						label="Exempt NSFW channels"
						onChange={(checked) =>
							am.updateRule(props.draft, "except_nsfw", checked)
						}
					/>
					<CheckboxOptionWithLabel
						id={`everyone-${ruleId()}`}
						seed={`everyone-${ruleId()}`}
						checked={includeEveryone()}
						label="Include everyone"
						description="whether this rule should affect everyone. actions aren't necessarily executed (eg. admins wont be timed out)"
						onChange={(checked) =>
							am.updateRule(props.draft, "include_everyone", checked)
						}
					/>
				</div>
			</Show>
		</div>
	);
};
