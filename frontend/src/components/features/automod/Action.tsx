import type { AutomodAction } from "ts-sdk";
import { useChannels } from "@/api";
import { ChannelPicker } from "@/atoms/ChannelPicker";
import { DurationInput } from "@/atoms/DurationInput";
import { type AutomodRuleDraft, useAutomod } from "./context";

export type ActionProps<T extends AutomodAction["type"]> = {
	draft: AutomodRuleDraft;
	index: number;
	action: AutomodAction & { type: T };
};

export const ActionBlock = (props: ActionProps<"Block">) => {
	const am = useAutomod();

	return (
		<div style="margin-top: 8px">
			<label>
				<h3 class="dim" style="margin:2px">
					Custom message (optional)
				</h3>
				<input
					type="text"
					placeholder="don't do that >:("
					value={props.action.message ?? ""}
					onInput={(e) =>
						am.updateAction(
							props.draft,
							props.index,
							"message",
							e.currentTarget.value,
						)
					}
				/>
			</label>
		</div>
	);
};

export const ActionTimeout = (props: ActionProps<"Timeout">) => {
	const am = useAutomod();

	return (
		<div style="margin-top: 8px">
			<label>
				<h3 class="dim" style="margin:2px">
					Duration
				</h3>
				<DurationInput
					value={props.action.duration / 1000}
					onInput={(seconds) => {
						if (seconds !== null && seconds !== "forever") {
							am.updateAction(
								props.draft,
								props.index,
								"duration",
								seconds * 1000,
							);
						}
					}}
				/>
			</label>
		</div>
	);
};

export const ActionRemove = () => {
	return (
		<p class="dim" style="margin-top: 8px">
			Message will be hidden but can be restored by moderators.
		</p>
	);
};

export const ActionSendAlert = (props: ActionProps<"SendAlert">) => {
	const am = useAutomod();
	const channels = useChannels();

	return (
		<div style="margin-top: 8px">
			<label>
				<h3 class="dim" style="margin:2px">
					Alert Channel
				</h3>
				<ChannelPicker
					selected={
						[...channels.cache.values()].find(
							(c) => c.id === props.action.channel_id,
						) ?? null
					}
					filter={(c) => c.type === "Text"}
					channels={() => [...channels.cache.values()]}
					onSelect={(channel) => {
						if (channel) {
							am.updateAction(
								props.draft,
								props.index,
								"channel_id",
								channel.id,
							);
						}
					}}
					placeholder="Select a channel..."
					required
				/>
			</label>
		</div>
	);
};
