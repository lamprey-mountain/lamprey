import { createSignal, For, type ParentProps } from "solid-js";
import { Modal } from "./mod";
import { useCtx } from "../context";
import { RadioDot } from "../icons";
import { flags } from "../flags";

export type ChannelTypeOption =
	| "Text"
	| "Voice"
	| "Category"
	| "Forum"
	| "Calendar";

interface ModalChannelCreateProps {
	room_id: string;
	cont: (data: { name: string; type: ChannelTypeOption }) => void;
}

export const ModalChannelCreate = (props: ModalChannelCreateProps) => {
	const ctx = useCtx()!;
	const [channelName, setChannelName] = createSignal("");
	const [channelType, setChannelType] = createSignal<ChannelTypeOption>("Text");

	const handleSubmit = (e: SubmitEvent) => {
		e.preventDefault();
		if (!channelName().trim()) return;

		props.cont({
			name: channelName().trim(),
			type: channelType(),
		});
		ctx.dispatch({ do: "modal.close" });
	};

	const handleCancel = () => {
		props.cont(null);
		ctx.dispatch({ do: "modal.close" });
	};

	return (
		<Modal>
			<h3>new channel</h3>
			<form class="new-channel" onSubmit={handleSubmit}>
				<h3 class="dim">
					channel type
				</h3>
				<div class="type">
					<For
						each={[
							{
								label: "text channel",
								type: "Text",
								description: "instant messaging",
							},
							{
								label: "voice channel",
								type: "Voice",
								description: "connect and talk",
							},
							{
								label: "category channel",
								type: "Category",
								description: "group other channels",
							},
							...(flags.has("channel_forum")
								? [{
									label: "forum channel",
									type: "Forum",
									description: "thread only channel",
								}]
								: []),
							...(flags.has("channel_calendar")
								? [{
									label: "calendar channel",
									type: "Calendar",
									description: "experiment, may be removed later",
								}]
								: []),
						]}
					>
						{(c) => (
							<label>
								<input
									type="radio"
									value={c.type}
									checked={channelType() === c.type}
									onInput={() => setChannelType(c.type)}
								/>
								<RadioDot checked={channelType() === c.type} />
								<div>
									<div>{c.label}</div>
									<div class="dim">{c.description}</div>
								</div>
							</label>
						)}
					</For>
				</div>

				<label style="display: block; margin-top: 12px">
					<h3 class="dim">channel name</h3>
					<input
						type="text"
						value={channelName()}
						onInput={(e) => setChannelName(e.currentTarget.value)}
						placeholder="talking"
						required
						autofocus
					/>
				</label>

				<div class="bottom">
					<button type="button" onClick={handleCancel}>
						Cancel
					</button>
					<button type="submit" class="primary">
						Create Channel
					</button>
				</div>
			</form>
		</Modal>
	);
};
