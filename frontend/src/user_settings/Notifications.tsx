import { createSignal, type VoidProps } from "solid-js";
import { type User } from "sdk";
import { Checkbox } from "../icons";

export function Notifications(_props: VoidProps<{ user: User }>) {
	// TODO: enable/disable desktop notifications
	// TODO: enable/disable push notifications
	// TODO: enable/disable tts notifications

	const [desktop, setDesktop] = createSignal(false);
	const [push, setPush] = createSignal(false);
	const [tts, setTts] = createSignal(false);

	return (
		<div class="user-settings-notifications">
			<h2>notifications</h2>
			<div class="options">
				<label class="option">
					<input
						type="checkbox"
						onInput={(e) => setDesktop(e.target.checked)}
					/>
					<Checkbox checked={desktop()} />
					<div>
						<div>Enable desktop notifications</div>
						<div class="dim"></div>
					</div>
				</label>
				<label class="option">
					<input
						type="checkbox"
						onInput={(e) => setPush(e.target.checked)}
					/>
					<Checkbox checked={push()} />
					<div>
						<div>Enable push notifications</div>
						<div class="dim"></div>
					</div>
				</label>
				<label class="option">
					<input
						type="checkbox"
						onInput={(e) => setTts(e.target.checked)}
					/>
					<Checkbox checked={tts()} />
					<div>
						<div>Enable text to speech for notifications</div>
						<div class="dim"></div>
					</div>
				</label>
			</div>
		</div>
	);
}
