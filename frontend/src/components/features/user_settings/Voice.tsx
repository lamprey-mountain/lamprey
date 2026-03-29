import { Show, type VoidProps } from "solid-js";
import type { User } from "sdk";
import { Checkbox } from "../../../icons";
import { useCtx } from "../../../context.ts";
import { Dropdown } from "../../../atoms/Dropdown";
import { CheckboxOption } from "../../../atoms/CheckboxOption";

export function Voice(_props: VoidProps<{ user: User }>) {
	const ctx = useCtx();

	// TODO: save input/output device volume, profile, etc per device id
	// TODO: automatic gain control

	const toggle = (setting: string) => () => {
		const c = ctx.preferences();
		ctx.setPreferences({
			...c,
			frontend: {
				...c.frontend,
				[setting]: c.frontend[setting] === "yes" ? "no" : "yes",
			},
		});
	};

	return (
		<div class="user-settings-voice">
			<h2>voice</h2>
			<br />
			<div style="display:flex;gap:4px">
				<div style="display:flex;flex-direction:column;flex:1">
					<h3 class="dim title2">input device</h3>
					<Dropdown
						selected={ctx.preferences().frontend["input_device"] || "default"}
						onSelect={(value) => {
							if (value) {
								const c = ctx.preferences();
								ctx.setPreferences({
									...c,
									frontend: {
										...c.frontend,
										input_device: value,
									},
								});
							}
						}}
						options={[
							{ item: "default", label: "Default Microphone" },
							{ item: "mic1", label: "Microphone 1" },
							{ item: "mic2", label: "Microphone 2" },
							{ item: "headset", label: "Headset Microphone" },
						]}
					/>
					<h3 class="dim title3">volume</h3>
					<input
						type="range"
						min="0"
						max="100"
						value={(ctx.preferences().frontend["mic_volume"] as
							| number
							| undefined) || 50}
						onChange={(e) => {
							const c = ctx.preferences();
							ctx.setPreferences({
								...c,
								frontend: {
									...c.frontend,
									["mic_volume"]: Number(e.target.value),
								},
							});
						}}
						class="slider volume"
					/>
				</div>
				<div style="display:flex;flex-direction:column;flex:1">
					<h3 class="dim title2">output device</h3>
					<Dropdown
						selected={ctx.preferences().frontend["output_device"] || "default"}
						onSelect={(value) => {
							if (value) {
								const c = ctx.preferences();
								ctx.setPreferences({
									...c,
									frontend: {
										...c.frontend,
										output_device: value,
									},
								});
							}
						}}
						options={[
							{ item: "default", label: "Default Speakers" },
							{ item: "speaker1", label: "Speakers 1" },
							{ item: "speaker2", label: "Speakers 2" },
							{ item: "headphones", label: "Headphones" },
							{ item: "headset", label: "Headset" },
						]}
					/>
					<h3 class="dim title3">volume</h3>
					<input
						type="range"
						min="0"
						max="100"
						value={(ctx.preferences().frontend["speaker_volume"] as
							| number
							| undefined) || 75}
						onChange={(e) => {
							const c = ctx.preferences();
							ctx.setPreferences({
								...c,
								frontend: {
									...c.frontend,
									["speaker_volume"]: Number(e.target.value),
								},
							});
						}}
						class="slider volume"
					/>
				</div>
			</div>
			<h3 class="dim title">mic check</h3>
			<div style="display:flex;gap:4px">
				<div style="flex:1;background:#111;border-radius:4px;overflow:hidden;">
					<div style="width:12%;background:oklch(var(--color-link-500));height:100%">
					</div>
				</div>
				<button>record</button>
				<button>play</button>
			</div>
			<h3 class="dim title">audio processing</h3>
			<CheckboxOption
				id={`user-${_props.user?.id ?? "@self"}-voice-echo-cancellation`}
				checked={ctx.preferences().frontend["voice_echo_cancellation"] ===
					"yes"}
				onChange={() => toggle("voice_echo_cancellation")()}
				seed={`user-${_props.user?.id ?? "@self"}-voice-echo-cancellation`}
			>
				<Checkbox
					checked={ctx.preferences().frontend["voice_echo_cancellation"] ===
						"yes"}
					seed={`user-${_props.user?.id ?? "@self"}-voice-echo-cancellation`}
				/>
				<span>Enable echo cancellation</span>
			</CheckboxOption>
			<CheckboxOption
				id={`user-${_props.user?.id ?? "@self"}-voice-noise-suppression`}
				checked={ctx.preferences().frontend["voice_noise_suppression"] ===
					"yes"}
				onChange={() => toggle("voice_noise_suppression")()}
				seed={`user-${_props.user?.id ?? "@self"}-voice-noise-suppression`}
			>
				<Checkbox
					checked={ctx.preferences().frontend["voice_noise_suppression"] ===
						"yes"}
					seed={`user-${_props.user?.id ?? "@self"}-voice-noise-suppression`}
				/>
				<span>Enable noise suppression</span>
			</CheckboxOption>
			<h3 class="dim title">activation</h3>
			<div class="options">
				<div class="option apart">
					<div>
						<div>Input mode</div>
						<div class="dim">Sensitivity for voice activation mode</div>
					</div>
					<Dropdown
						selected={ctx.preferences().frontend["voice_input_mode"] || "vad"}
						onSelect={(value) => {
							if (value) {
								const c = ctx.preferences();
								ctx.setPreferences({
									...c,
									frontend: {
										...c.frontend,
										voice_input_mode: value,
									},
								});
							}
						}}
						options={[
							{ item: "vad", label: "Voice activity" },
							{ item: "ptt", label: "Push to talk" },
							{ item: "open", label: "Open mic" },
						]}
					/>
				</div>
				<Show
					when={(ctx.preferences().frontend["voice_input_mode"] || "vad") ===
						"vad"}
				>
					<div class="option apart">
						<div>
							<div>Voice activity threshold</div>
							<div class="dim">Sensitivity for voice activation mode</div>
						</div>
						<input
							type="range"
							min="0"
							max="100"
							value={(ctx.preferences().frontend["voice_activity_threshold"] as
								| number
								| undefined) ||
								30}
							onChange={(e) => {
								const c = ctx.preferences();
								ctx.setPreferences({
									...c,
									frontend: {
										...c.frontend,
										["voice_activity_threshold"]: Number(e.target.value),
									},
								});
							}}
							class="slider"
						/>
					</div>
					<div class="option apart">
						<div>
							<div>Voice activity timeout</div>
							<div class="dim">How long of silence before deactivation</div>
						</div>
						<input
							type="range"
							min="0"
							max="5000"
							step="100"
							value={(ctx.preferences().frontend["voice_activity_timeout"] as
								| number
								| undefined) ||
								1000}
							onChange={(e) => {
								const c = ctx.preferences();
								ctx.setPreferences({
									...c,
									frontend: {
										...c.frontend,
										["voice_activity_timeout"]: Number(e.target.value),
									},
								});
							}}
							class="slider"
						/>
					</div>
				</Show>
			</div>
		</div>
	);
}
