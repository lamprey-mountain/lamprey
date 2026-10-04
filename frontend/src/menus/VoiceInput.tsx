import { createSignal, For } from "solid-js";
import { Checkbox } from "@/atoms/icons";
import { useNavigate } from "@/contexts/router";
import { Item, Menu, Separator } from "./Parts";

export const VoiceInputMenu = () => {
	const nav = useNavigate();
	// const [voice, voiceActions] = useVoice();

	// TODO: integrate with actual voice settings
	// TODO: allow going above 100%
	// TODO: deduplicate code with VoiceOutput
	const [volume, setVolume] = createSignal(1);
	const [inputDevice, setInputDevice] = createSignal("Default Input Device");

	return (
		<Menu>
			<h3 class="dim" style="padding:2px 8px;">
				Input Device
			</h3>
			<For each={["Default Input Device", "Microphone 1"]}>
				{(a) => (
					<Item onClick={() => setInputDevice(a)}>
						<div style="display: flex; align-items: start; gap: 4px">
							<Checkbox
								checked={a === inputDevice()} // PERF: createSelector
								seed={`menu-voice-input-${a}`}
							/>
							<div style="margin: 2px 0">{a}</div>
						</div>
					</Item>
				)}
			</For>
			<Separator />
			<li onClick={(e) => e.stopPropagation()}>
				<label style="display:block;padding:0 8px;padding-top:8px">
					<h3 class="dim">
						Input Volume -{" "}
						<span style="color:oklch(var(--color-fg1))">
							{(volume() * 100).toFixed(1)}%
						</span>{" "}
					</h3>
					<input
						type="range"
						min="0"
						max="1"
						step="any"
						// list="volume-detents"
						value={volume()}
						onInput={(e) => {
							setVolume(e.target.valueAsNumber);
						}}
					/>
				</label>
			</li>
			<Separator />
			<Item onClick={() => nav("/settings/voice")}>Voice Settings</Item>
		</Menu>
	);
};
