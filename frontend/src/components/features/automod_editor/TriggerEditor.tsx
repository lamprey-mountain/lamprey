import { createEffect, For, Match, Switch } from "solid-js";
import { createSignal } from "solid-js";
import type { AutomodTrigger } from "sdk";

export interface TriggerEditorProps {
	trigger: AutomodTrigger;
	updateTriggerValue: (key: string, val: any) => void;
	updateTriggerType: (type: AutomodTrigger["type"]) => void;
}

// --- 1. Specific Trigger Field Editors ---

function TextKeywordsFields(props: {
	trigger: AutomodTrigger & { type: "TextKeywords" };
	update: (key: string, val: any) => void;
}) {
	const [keywordsRaw, setKeywordsRaw] = createSignal(
		props.trigger.keywords?.join("\n") || "",
	);

	createEffect(() => {
		setKeywordsRaw(props.trigger.keywords?.join("\n") || "");
	});

	return (
		<label>
			Keywords (one per line)
			<textarea
				rows={5}
				placeholder="Enter keywords..."
				value={keywordsRaw()}
				onInput={(e) => {
					const val = e.currentTarget.value;
					setKeywordsRaw(val);
					props.update("keywords", val.split("\n"));
				}}
				onBlur={() => {
					const clean = keywordsRaw()
						.split("\n")
						.map((k) => k.trim())
						.filter((k) => k !== "");
					setKeywordsRaw(clean.join("\n"));
					props.update("keywords", clean);
				}}
			/>
		</label>
	);
}

function TextRegexFields(props: {
	trigger: AutomodTrigger & { type: "TextRegex" };
	update: (key: string, val: any) => void;
}) {
	const [denyRaw, setDenyRaw] = createSignal(
		props.trigger.deny?.join("\n") || "",
	);
	const [allowRaw, setAllowRaw] = createSignal(
		props.trigger.allow?.join("\n") || "",
	);

	createEffect(() => {
		setDenyRaw(props.trigger.deny?.join("\n") || "");
		setAllowRaw(props.trigger.allow?.join("\n") || "");
	});

	return (
		<div style={{ display: "flex", "flex-direction": "column", gap: "10px" }}>
			<label>
				Blocked Patterns (Regex)
				<textarea
					placeholder="e.g. [0-9]{10}"
					value={denyRaw()}
					onInput={(e) => {
						const val = e.currentTarget.value;
						setDenyRaw(val);
						props.update("deny", val.split("\n"));
					}}
					onBlur={() => {
						const clean = denyRaw()
							.split("\n")
							.map((d) =>
								d.trim()
							)
							.filter((d) => d !== "");
						setDenyRaw(clean.join("\n"));
						props.update("deny", clean);
					}}
				/>
			</label>
			<label>
				Allowed Patterns (Regex)
				<textarea
					placeholder="Patterns to ignore..."
					value={allowRaw()}
					onInput={(e) => {
						const val = e.currentTarget.value;
						setAllowRaw(val);
						props.update("allow", val.split("\n"));
					}}
					onBlur={() => {
						const clean = allowRaw()
							.split("\n")
							.map((a) => a.trim())
							.filter((a) => a !== "");
						setAllowRaw(clean.join("\n"));
						props.update("allow", clean);
					}}
				/>
			</label>
		</div>
	);
}

function TextLinksFields(props: {
	trigger: AutomodTrigger & { type: "TextLinks" };
	update: (key: string, val: any) => void;
}) {
	const [hostnamesRaw, setHostnamesRaw] = createSignal(
		props.trigger.hostnames?.join("\n") || "",
	);

	createEffect(() => {
		setHostnamesRaw(props.trigger.hostnames?.join("\n") || "");
	});

	return (
		<div style={{ display: "flex", "flex-direction": "column", gap: "10px" }}>
			<label style={{ display: "flex", "align-items": "center", gap: "8px" }}>
				<input
					type="checkbox"
					checked={props.trigger.whitelist || false}
					onChange={(e) =>
						props.update("whitelist", e.currentTarget.checked)}
				/>
				Is Whitelist (only allow these domains)
			</label>
			<label>
				Hostnames (one per line)
				<textarea
					placeholder="example.com"
					value={hostnamesRaw()}
					onInput={(e) => {
						const val = e.currentTarget.value;
						setHostnamesRaw(val);
						props.update("hostnames", val.split("\n"));
					}}
					onBlur={() => {
						const clean = hostnamesRaw()
							.split("\n")
							.map((h) => h.trim())
							.filter((h) => h !== "");
						setHostnamesRaw(clean.join("\n"));
						props.update("hostnames", clean);
					}}
				/>
			</label>
		</div>
	);
}

function TextBuiltinFields(props: {
	trigger: AutomodTrigger & { type: "TextBuiltin" };
	update: (key: string, val: any) => void;
}) {
	return (
		<label>
			Built-in List Name
			<input
				type="text"
				value={props.trigger.list || ""}
				onInput={(e) => props.update("list", e.currentTarget.value)}
			/>
		</label>
	);
}

function MediaScanFields(props: {
	trigger: AutomodTrigger & { type: "MediaScan" };
	update: (key: string, val: any) => void;
}) {
	return (
		<label>
			Scanner Type
			<select
				value={props.trigger.scanner || ""}
				onChange={(e) => props.update("scanner", e.currentTarget.value)}
			>
				<option value="Nsfw">NSFW Detection</option>
				<option value="Malware">Malware / Links</option>
			</select>
		</label>
	);
}

// --- 2. The Main Trigger Editor ---

export function TriggerEditor(props: TriggerEditorProps) {
	const types: AutomodTrigger["type"][] = [
		"TextKeywords",
		"TextRegex",
		"TextLinks",
		"TextBuiltin",
		"MediaScan",
	];

	return (
		<div
			class="trigger-editor"
			style={{ display: "flex", "flex-direction": "column", gap: "10px" }}
		>
			<label>
				<strong>Trigger Type</strong>
				<select
					value={props.trigger.type}
					onChange={(e) =>
						props.updateTriggerType(
							e.currentTarget.value as AutomodTrigger["type"],
						)}
				>
					<For each={types}>{(t) => <option value={t}>{t}</option>}</For>
				</select>
			</label>

			<div class="trigger-specific-settings">
				<Switch>
					<Match when={props.trigger.type === "TextKeywords"}>
						<TextKeywordsFields
							trigger={props.trigger as AutomodTrigger & {
								type: "TextKeywords";
							}}
							update={props.updateTriggerValue}
						/>
					</Match>
					<Match when={props.trigger.type === "TextRegex"}>
						<TextRegexFields
							trigger={props.trigger as AutomodTrigger & { type: "TextRegex" }}
							update={props.updateTriggerValue}
						/>
					</Match>
					<Match when={props.trigger.type === "TextLinks"}>
						<TextLinksFields
							trigger={props.trigger as AutomodTrigger & { type: "TextLinks" }}
							update={props.updateTriggerValue}
						/>
					</Match>
					<Match when={props.trigger.type === "TextBuiltin"}>
						<TextBuiltinFields
							trigger={props.trigger as AutomodTrigger & {
								type: "TextBuiltin";
							}}
							update={props.updateTriggerValue}
						/>
					</Match>
					<Match when={props.trigger.type === "MediaScan"}>
						<MediaScanFields
							trigger={props.trigger as AutomodTrigger & { type: "MediaScan" }}
							update={props.updateTriggerValue}
						/>
					</Match>
				</Switch>
			</div>
		</div>
	);
}
