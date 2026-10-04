import type { User } from "sdk";
import {
	createMemo,
	createSignal,
	onCleanup,
	onMount,
	Show,
	type VoidProps,
} from "solid-js";
import { useCtx } from "@/app/context";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Dropdown } from "@/atoms/Dropdown";
import { Checkbox } from "@/atoms/icons";
import { createVAD } from "../voice/vad";

// NOTE: maybe move this to another file?
const useMediaDevices = () => {
	const [devices, setDevices] = createSignal([] as MediaDeviceInfo[]);

	const formatKind = (kind: MediaDeviceKind) => {
		switch (kind) {
			case "audioinput":
				return "microphone";
			case "audiooutput":
				return "output";
			case "videoinput":
				return "camera";
		}
	};

	const update = (devices: MediaDeviceInfo[]) => {
		const mapped = devices.map((d) => {
			return {
				groupId: d.groupId,
				deviceId: d.deviceId || "default",
				kind: d.kind,
				label: d.label || `Default ${formatKind(d.kind)}`,
			} as MediaDeviceInfo;
		});
		console.log("UPDATE", mapped);
		setDevices(mapped);
	};

	// browsers don't reveal any information unless we already have a stream
	navigator.mediaDevices
		.getUserMedia({ audio: true })
		.then((stream) => {
			navigator.mediaDevices
				.enumerateDevices()
				.then(update)
				.finally(() => {
					stream.getTracks().forEach((t) => t.stop());
				});
		})
		.catch((err) => console.error("getUserMedia failed", err));

	const refetch = () => {
		navigator.mediaDevices.enumerateDevices().then(update);
	};

	refetch();
	navigator.mediaDevices.addEventListener("devicechange", refetch);
	onCleanup(() =>
		navigator.mediaDevices.removeEventListener("devicechange", refetch),
	);

	return devices;
};

const createMicCheck = () => {
	const vad = createVAD();
	const [active, setActive] = createSignal(false);
	let stream: MediaStream | undefined;

	const stop = () => {
		if (stream) {
			stream.getTracks().forEach((t) => t.stop());
		}
		stream = undefined;
		setActive(false);
	};

	const start = async (deviceId?: string) => {
		// don't reuse voice context's state, we don't want to accidentally unmute the user if they're in a voice channel
		stream = await navigator.mediaDevices.getUserMedia(
			deviceId ? { audio: { deviceId } } : { audio: true },
		);
		vad.connect(stream);
		setActive(true);
	};

	const level = createMemo((prev: number = 0) => {
		const db = 20 * Math.log10(Math.max(vad.rms(), 1e-4)); // -80..0
		const target = Math.min(1, Math.max(0, (db + 60) / 60)); // -60dB..0dB -> 0..1
		return target > prev ? target : prev * 0.85 + target * 0.15; // immediate attack, smooth release
	});

	onCleanup(stop);

	return {
		active,
		stop,
		start,
		level,
		vad,
	};
};

export function Voice(_props: VoidProps<{ user: User }>) {
	const ctx = useCtx();
	const devices = useMediaDevices();
	const micCheck = createMicCheck();

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
						selected={
							ctx.preferences().frontend.input_device ||
							devices().find((d) => d.kind === "audioinput")?.deviceId ||
							"default"
						}
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
						options={devices()
							.filter((i) => i.kind === "audioinput")
							.map((i) => ({ item: i.deviceId, label: i.label }))}
					/>
					<h3 class="dim title3">volume</h3>
					<input
						type="range"
						min="0"
						max="100"
						value={
							(ctx.preferences().frontend.mic_volume as number | undefined) ||
							50
						}
						onChange={(e) => {
							const c = ctx.preferences();
							ctx.setPreferences({
								...c,
								frontend: {
									...c.frontend,
									mic_volume: Number(e.target.value),
								},
							});
						}}
						class="slider volume"
					/>
				</div>
				<div style="display:flex;flex-direction:column;flex:1">
					<h3 class="dim title2">output device</h3>
					<Dropdown
						selected={
							ctx.preferences().frontend.output_device ||
							devices().find((d) => d.kind === "audiooutput")?.deviceId ||
							"default"
						}
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
						options={devices()
							.filter((i) => i.kind === "audiooutput")
							.map((i) => ({ item: i.deviceId, label: i.label }))}
					/>
					<h3 class="dim title3">volume</h3>
					<input
						type="range"
						min="0"
						max="100"
						value={
							(ctx.preferences().frontend.speaker_volume as
								| number
								| undefined) || 75
						}
						onChange={(e) => {
							const c = ctx.preferences();
							ctx.setPreferences({
								...c,
								frontend: {
									...c.frontend,
									speaker_volume: Number(e.target.value),
								},
							});
						}}
						class="slider volume"
					/>
				</div>
			</div>
			<h3 class="dim title">mic check</h3>
			<div class="mic-check" classList={{ active: micCheck.active() }}>
				{/* TODO(?): allow recording and replaying your audio? */}
				<button
					type="button"
					class="button primary"
					onClick={() => {
						micCheck.active() ? micCheck.stop() : micCheck.start();
					}}
				>
					<div class="inner">{micCheck.active() ? "Stop" : "Test"}</div>
				</button>
				<div class="tape">
					<div
						class="level"
						style={{
							"--level": `${micCheck.level() * 100}%`,
							background: micCheck.vad.hasVoiceActivity()
								? "oklch(var(--color-link-500))"
								: "oklch(var(--color-bg4))",
						}}
					></div>
					<div class="text">{micCheck.active() ? "" : "no signal"}</div>
				</div>
			</div>
			<h3 class="dim title">audio processing</h3>
			<CheckboxOption
				id={`user-${_props.user?.id ?? "@self"}-voice-echo-cancellation`}
				checked={ctx.preferences().frontend.voice_echo_cancellation === "yes"}
				onChange={() => toggle("voice_echo_cancellation")()}
				seed={`user-${_props.user?.id ?? "@self"}-voice-echo-cancellation`}
			>
				<Checkbox
					checked={ctx.preferences().frontend.voice_echo_cancellation === "yes"}
					seed={`user-${_props.user?.id ?? "@self"}-voice-echo-cancellation`}
				/>
				<span>Enable echo cancellation</span>
			</CheckboxOption>
			<CheckboxOption
				id={`user-${_props.user?.id ?? "@self"}-voice-noise-suppression`}
				checked={ctx.preferences().frontend.voice_noise_suppression === "yes"}
				onChange={() => toggle("voice_noise_suppression")()}
				seed={`user-${_props.user?.id ?? "@self"}-voice-noise-suppression`}
			>
				<Checkbox
					checked={ctx.preferences().frontend.voice_noise_suppression === "yes"}
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
						selected={ctx.preferences().frontend.voice_input_mode || "vad"}
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
					when={
						(ctx.preferences().frontend.voice_input_mode || "vad") === "vad"
					}
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
							value={
								(ctx.preferences().frontend.voice_activity_threshold as
									| number
									| undefined) || 30
							}
							onChange={(e) => {
								const c = ctx.preferences();
								ctx.setPreferences({
									...c,
									frontend: {
										...c.frontend,
										voice_activity_threshold: Number(e.target.value),
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
							value={
								(ctx.preferences().frontend.voice_activity_timeout as
									| number
									| undefined) || 1000
							}
							onChange={(e) => {
								const c = ctx.preferences();
								ctx.setPreferences({
									...c,
									frontend: {
										...c.frontend,
										voice_activity_timeout: Number(e.target.value),
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
