import { For, Match, Show, Switch } from "solid-js";
import type { AutomodAction, AutomodTarget, Channel } from "sdk";
import { ChannelIcon } from "../../../avatar/ChannelIcon.tsx";
import { ChannelPicker } from "../../../atoms/ChannelPicker.tsx";

export interface ActionsEditorProps {
	target: AutomodTarget;
	actions: AutomodAction[];
	updateActionValue: (idx: number, key: string, val: unknown) => void;
	updateActionType: (idx: number, type: AutomodAction["type"]) => void;
	addAction: () => void;
	removeAction: (idx: number) => void;
	room_id: string;
	channels: () => Channel[];
}

// --- 1. Specific Field Editors ---

function BlockFields(props: {
	action: AutomodAction & { type: "Block" };
	update: (key: string, val: unknown) => void;
}) {
	return (
		<label>
			Custom Message (optional)
			<input
				type="text"
				placeholder="e.g. Please avoid using that word."
				value={props.action.message || ""}
				onInput={(e) => props.update("message", e.currentTarget.value)}
			/>
		</label>
	);
}

function TimeoutFields(props: {
	action: AutomodAction & { type: "Timeout" };
	update: (key: string, val: unknown) => void;
}) {
	// Convert ms to minutes for a better UX
	const mins = () => Math.floor(props.action.duration / 60000);
	return (
		<label>
			Duration (minutes)
			<input
				type="number"
				value={mins()}
				onInput={(e) =>
					props.update("duration", parseInt(e.currentTarget.value) * 60000)}
			/>
		</label>
	);
}

function SendAlertFields(props: {
	action: AutomodAction & { type: "SendAlert" };
	update: (key: string, val: unknown) => void;
	room_id: string;
	channels: () => Channel[];
}) {
	const findChannel = () =>
		props.channels().find((c) => c.id === props.action.channel_id);

	return (
		<label>
			Alert Channel
			<ChannelPicker
				selected={findChannel() ?? null}
				channels={props.channels}
				filter={(c) => c.room_id === props.room_id}
				onSelect={(channel) => props.update("channel_id", channel?.id ?? "")}
				placeholder="Select a channel..."
			/>
		</label>
	);
}

function RemoveFields() {
	return (
		<p class="hint">
			Message will be hidden but can be restored by moderators.
		</p>
	);
}

// --- 2. The Row Wrapper ---
// Handles the common UI like the dropdown and delete button
function ActionItem(props: {
	action: AutomodAction;
	index: number;
	target: AutomodTarget;
	onUpdateType: (type: AutomodAction["type"]) => void;
	onUpdateValue: (key: string, val: unknown) => void;
	onRemove: () => void;
	room_id: string;
	channels: () => Channel[];
}) {
	// Helper to determine if an action type is allowed for the current target
	const isActionAllowed = (type: AutomodAction["type"]) => {
		if (props.target === "Member") {
			// Per Rust comments: Timeout and Remove are for Content only
			return type === "Block" || type === "SendAlert";
		}
		return true;
	};

	// Human-readable labels based on context
	const getActionLabel = (type: AutomodAction["type"]) => {
		const labels: Record<string, { Content: string; Member: string }> = {
			Block: {
				Content: "Block Message",
				Member: "Block / Reset Profile Update",
			},
			Timeout: {
				Content: "Timeout User",
				Member: "Timeout (Not available for Member)",
			},
			Remove: {
				Content: "Remove Message",
				Member: "Remove (Not available for Member)",
			},
			SendAlert: {
				Content: "Send Alert to Channel",
				Member: "Send Alert to Channel",
			},
		};
		return labels[type][props.target as keyof typeof labels[string]] || type;
	};

	return (
		<div
			class="action-item-card"
			style={{
				border: "1px solid oklch(var(--color-sep-300))",
				padding: "10px",
			}}
		>
			<header
				style={{
					display: "flex",
					gap: "10px",
					"align-items": "center",
				}}
			>
				<select
					value={props.action.type}
					onChange={(e) =>
						props.onUpdateType(e.currentTarget.value as AutomodAction["type"])}
				>
					<Show when={isActionAllowed("Block")}>
						<option value="Block">{getActionLabel("Block")}</option>
					</Show>
					<Show when={isActionAllowed("Timeout")}>
						<option value="Timeout">{getActionLabel("Timeout")}</option>
					</Show>
					<Show when={isActionAllowed("Remove")}>
						<option value="Remove">{getActionLabel("Remove")}</option>
					</Show>
					<Show when={isActionAllowed("SendAlert")}>
						<option value="SendAlert">{getActionLabel("SendAlert")}</option>
					</Show>
				</select>

				<button
					type="button"
					class="error ghost small"
					onClick={props.onRemove}
				>
					Remove
				</button>
			</header>

			<div class="action-fields" style={{ "margin-top": "10px" }}>
				<Switch>
					<Match when={props.action.type === "Block"}>
						<BlockFields
							action={props.action as AutomodAction & { type: "Block" }}
							update={props.onUpdateValue}
						/>
					</Match>
					<Match when={props.action.type === "Timeout"}>
						<TimeoutFields
							action={props.action as AutomodAction & { type: "Timeout" }}
							update={props.onUpdateValue}
						/>
					</Match>
					<Match when={props.action.type === "SendAlert"}>
						<SendAlertFields
							action={props.action as AutomodAction & { type: "SendAlert" }}
							update={props.onUpdateValue}
							room_id={props.room_id}
							channels={props.channels}
						/>
					</Match>
					<Match when={props.action.type === "Remove"}>
						<RemoveFields />
					</Match>
				</Switch>
			</div>
		</div>
	);
}

// --- 3. The Main List Editor ---
export function ActionsEditor(props: ActionsEditorProps) {
	return (
		<div class="actions-editor">
			<header
				style={{
					display: "flex",
					"justify-content": "space-between",
					"align-items": "center",
				}}
			>
				<h4>Actions</h4>
				<button type="button" class="small" onClick={props.addAction}>
					+ Add Action
				</button>
			</header>

			<div
				class="actions-list"
				style={{
					display: "flex",
					"flex-direction": "column",
					gap: "10px",
					"margin-top": "10px",
				}}
			>
				<For each={props.actions}>
					{(action, i) => (
						<ActionItem
							action={action}
							index={i()}
							target={props.target}
							onUpdateType={(type) => props.updateActionType(i(), type)}
							onUpdateValue={(key, val) =>
								props.updateActionValue(i(), key, val)}
							onRemove={() => props.removeAction(i())}
							room_id={props.room_id}
							channels={props.channels}
						/>
					)}
				</For>
			</div>

			<Show when={props.actions.length === 0}>
				<p class="hint">No actions configured. This rule will do nothing.</p>
			</Show>
		</div>
	);
}
