import { createSignal, For } from "solid-js";
import { Checkbox } from "@/atoms/icons";
import { useNavigate } from "@/contexts/router";
import { Item, Menu, Separator } from "./Parts";

export const VoiceOutputMenu = () => {
	const nav = useNavigate();
	// const [voice, voiceActions] = useVoice();

	// TODO: integrate with actual voice settings
	// TODO: allow going above 100%
	// TODO: deduplicate code with VoiceInput
	// TODO: maybe add ptt toggle and settings that are useful to configure quickly?
	const [volume, setVolume] = createSignal(1);
	const [outputDevice, setOutputDevice] = createSignal("Default Output Device");

	return (
		<Menu>
			<h3 class="dim" style="padding:2px 8px;">
				Output Device
			</h3>
			<For each={["Default Output Device", "Headphones 1"]}>
				{(a) => (
					<Item onClick={() => setOutputDevice(a)}>
						<div style="display: flex; align-items: start; gap: 4px">
							<Checkbox
								checked={a === outputDevice()} // PERF: createSelector
								seed={`menu-voice-output-${a}`}
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
						Output Volume -{" "}
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
