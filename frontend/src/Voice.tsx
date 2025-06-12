import { Room, Thread } from "sdk";
import { createSignal } from "solid-js";
import iconCamera from "./assets/camera.png";
import iconHeadphones from "./assets/headphones.png";
import iconMic from "./assets/mic.png";
import iconScreenshare from "./assets/screenshare.png";
import iconSettings from "./assets/settings.png";
import iconX from "./assets/x.png";

export const Voice = (p: { room: Room; thread: Thread }) => {
	const [muted, setMuted] = createSignal(false);
	const [deafened, setDeafened] = createSignal(false);

	// TODO: tooltips
	// const [tip, setTip] = createSignal("some text here")
	// const tooltip = createTooltip({
	// 	tip: () => tip(),
	// });

	const handleMouseOver = (e: MouseEvent) => {
		// const tipEl = ((e.target as HTMLElement).closest("[data-tooltip]") as HTMLElement);
		// if (!tipEl) return;
		// const tipText = tipEl.dataset.tooltip;
		// setTip(tipText as string);
		// tooltip.setContentEl(tipEl)
		// tooltip.showTip();
	};

	const handleMouseOut = (e: MouseEvent) => {
		// const tipEl = ((e.target as HTMLElement).closest("[data-tooltip]") as HTMLElement);
		// if (tipEl) return;
		// tooltip.considerHidingTip()
	};

	return (
		<div class="webrtc">
			<div
				class="ui"
				onMouseOver={handleMouseOver}
				onMouseOut={handleMouseOut}
			>
				<div class="row">
					<div style="flex:1">
						<div style="color:green">
							connected
						</div>
						<div>
							room / thread
						</div>
					</div>
					<div>
						<button data-tooltip="arst">
							{/* camera */}
							<img class="icon" src={iconCamera} />
						</button>
						<button>
							{/* camera */}
							<img class="icon" src={iconScreenshare} />
						</button>
						<button>
							{/* disconnect */}
							<img class="icon" src={iconX} />
						</button>
					</div>
				</div>
				<div class="row toolbar">
					<div style="flex:1">user</div>
					<button onClick={() => setMuted((m) => !m)}>
						{/* mute */}
						<ToggleIcon checked={muted()} src={iconMic} />
					</button>
					<button onClick={() => setDeafened((d) => !d)}>
						{/* deafen */}
						<ToggleIcon checked={deafened()} src={iconHeadphones} />
					</button>
					<button onClick={() => alert("todo")}>
						{/* settings */}
						<img class="icon" src={iconSettings} />
					</button>
				</div>
			</div>
		</div>
	);
};

const ToggleIcon = (props: { checked: boolean; src: string }) => {
	return (
		<svg
			viewBox={`0 0 64 64`}
			role="img"
			class="icon strike"
			aria-checked={props.checked}
		>
			<defs>
				<mask id="strike">
					<rect width="64" height="64" fill="white" />
					<line
						x1="0"
						y1="0"
						x2="64"
						y2="64"
						stroke="black"
						stroke-width="32"
					/>
				</mask>
			</defs>
			<image href={props.src} />
			<line class="line" x1="8" y1="8" x2="56" y2="56" stroke-width="8" />
		</svg>
	);
};
