import { createEffect, on } from "solid-js";
import type { AutomodTrigger } from "ts-sdk";
import { CheckboxOptionWithLabel } from "@/atoms/CheckboxOption";
import { Dropdown } from "@/atoms/Dropdown";
import { type AutomodRuleDraft, useAutomod } from "./context";

const KeywordsTextarea = (props: {
	label: string;
	value: string;
	onChange: (value: string) => void;
	placeholder?: string;
}) => {
	let editing = false;
	let textareaEl!: HTMLTextAreaElement;

	// update text when value changes and not editing
	createEffect(
		on(
			() => props.value,
			(val) => {
				if (!editing) {
					textareaEl.value = val;
				}
			},
		),
	);

	return (
		<label style="display: block; margin-top: 8px">
			<h3 class="dim" style="margin:2px">
				{props.label}
			</h3>
			<textarea
				ref={textareaEl}
				class="textarea"
				onInput={(e) => {
					const t = e.currentTarget.value;
					if (t === props.value) return;
					props.onChange(t);
				}}
				onFocus={() => (editing = true)}
				onBlur={() => (editing = false)}
				placeholder={props.placeholder}
			/>
		</label>
	);
};

export const TriggerKeywords = (props: {
	draft: AutomodRuleDraft;
	trigger: AutomodTrigger & { type: "TextKeywords" };
}) => {
	const am = useAutomod();

	const updateTrigger = (key: string, value: string[]) => {
		const newTrigger = { ...props.trigger, [key]: value };
		am.updateRule(props.draft, "trigger", newTrigger);
	};

	return (
		<div>
			<KeywordsTextarea
				label="Denied Keywords (one per line)"
				value={props.trigger.keywords.join("\n")}
				onChange={(val) =>
					updateTrigger(
						"keywords",
						val
							.split("\n")
							.map((i) => i.trim())
							.filter((i) => i),
					)
				}
				placeholder={"pickles\ndill"}
			/>
			<KeywordsTextarea
				label="Allowed keywords (one per line. overwrites denied keywords)"
				value={props.trigger.allow.join("\n")}
				onChange={(val) =>
					updateTrigger(
						"allow",
						val
							.split("\n")
							.map((i) => i.trim())
							.filter((i) => i),
					)
				}
				placeholder="vinegar"
			/>
		</div>
	);
};

export const TriggerRegex = (props: {
	draft: AutomodRuleDraft;
	trigger: AutomodTrigger & { type: "TextRegex" };
}) => {
	const am = useAutomod();

	const updateTrigger = (key: string, value: string[]) => {
		const newTrigger = { ...props.trigger, [key]: value };
		am.updateRule(props.draft, "trigger", newTrigger);
	};

	return (
		<div>
			<KeywordsTextarea
				label="Denied Patterns (Regex, one per line)"
				value={props.trigger.deny.join("\n")}
				onChange={(val) =>
					updateTrigger(
						"deny",
						val
							.split("\n")
							.map((i) => i.trim())
							.filter((i) => i),
					)
				}
				placeholder="[0-9]{10}"
			/>
			<KeywordsTextarea
				label="Allowed Patterns (Regex, one per line)"
				value={props.trigger.allow.join("\n")}
				onChange={(val) =>
					updateTrigger(
						"allow",
						val
							.split("\n")
							.map((i) => i.trim())
							.filter((i) => i),
					)
				}
				placeholder="[0-9]{5}"
			/>
		</div>
	);
};

export const TriggerLinks = (props: {
	draft: AutomodRuleDraft;
	trigger: AutomodTrigger & { type: "TextLinks" };
}) => {
	const am = useAutomod();

	const updateTrigger = (key: string, value: any) => {
		const newTrigger = { ...props.trigger, [key]: value };
		am.updateRule(props.draft, "trigger", newTrigger);
	};

	const ruleId = () =>
		props.draft.state === "create" ? props.draft.nonce : props.draft.rule.id;

	return (
		<div style="margin-top: 8px">
			<CheckboxOptionWithLabel
				id={`whitelist-${ruleId()}`}
				seed={`whitelist-${ruleId()}`}
				checked={props.trigger.whitelist ?? false}
				label="whitelist"
				description="instead of blocking these domains, only allow these domains"
				onChange={(checked) => updateTrigger("whitelist", checked)}
			/>
			<KeywordsTextarea
				label="Hostnames (one per line)"
				value={props.trigger.hostnames.join("\n")}
				onChange={(val) =>
					updateTrigger(
						"hostnames",
						val
							.split("\n")
							.map((i) => i.trim())
							.filter((i) => i),
					)
				}
				placeholder="example.com"
			/>
		</div>
	);
};

export const TriggerBuiltin = (props: {
	draft: AutomodRuleDraft;
	trigger: AutomodTrigger & { type: "TextBuiltin" };
}) => {
	const am = useAutomod();

	const updateTrigger = (key: string, value: string) => {
		const newTrigger = { ...props.trigger, [key]: value };
		am.updateRule(props.draft, "trigger", newTrigger);
	};

	return (
		<label style="display: block; margin-top: 8px">
			<h3 class="dim" style="margin:2px">
				Built-in List Name
			</h3>
			<Dropdown
				options={[
					{ item: "Profanity", label: "Profanity" },
					{ item: "Spam", label: "Spam" },
				]}
				onSelect={(item) => updateTrigger("list", item)}
				selected={props.trigger.list}
			/>
		</label>
	);
};

export const TriggerMediaScan = (props: {
	draft: AutomodRuleDraft;
	trigger: AutomodTrigger & { type: "MediaScan" };
}) => {
	const am = useAutomod();

	const updateTrigger = (key: string, value: string) => {
		const newTrigger = { ...props.trigger, [key]: value };
		am.updateRule(props.draft, "trigger", newTrigger);
	};

	return (
		<label style="display: block; margin-top: 8px">
			<h3 class="dim" style="margin:2px">
				Scanner Type
			</h3>
			<Dropdown
				options={[
					{ item: "Nsfw", label: "NSFW Detection" },
					{ item: "Malware", label: "Malware / Links" },
				]}
				onSelect={(item) => updateTrigger("scanner", item!)}
				selected={props.trigger.scanner}
				required
			/>
		</label>
	);
};
