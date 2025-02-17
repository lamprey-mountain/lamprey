import { createEffect, createSignal, onCleanup, ValidComponent } from "solid-js";
import iconPlay from "../assets/play.png";
import iconPause from "../assets/pause.png";
import iconVolumeLow from "../assets/volume-low.png";
import iconVolumeMedium from "../assets/volume-medium.png";
import iconVolumeHigh from "../assets/volume-high.png";
import iconVolumeMute from "../assets/volume-mute.png";
import iconVolumeMax from "../assets/volume-max.png";
import { formatTime, getDuration, MediaProps } from "./util.ts";
import { tooltip } from "../Tooltip.tsx";

export const AudioView = (props: MediaProps) => {
	// NOTE: not using audio element so i can keep audio alive while scrolling (will impl later)
	const audio = new globalThis.Audio();
	createEffect(() => audio.src = props.media.source.url);
	onCleanup(() => audio.pause());

	const [duration, setDuration] = createSignal(getDuration(props.media));
	const [progress, setProgress] = createSignal(0);
	const [progressPreview, setProgressPreview] = createSignal<null | number>(
		null,
	);
	const [playing, setPlaying] = createSignal(false);
	const [volume, setVolume] = createSignal(1);
	const [muted, setMuted] = createSignal(false);

	audio.ondurationchange = () => setDuration(audio.duration);
	audio.ontimeupdate = () => setProgress(audio.currentTime);
	audio.onplay = () => setPlaying(true);
	audio.onpause = () => setPlaying(false);

	createEffect(() => audio.muted = muted());
	createEffect(() => audio.volume = volume());

	const togglePlayPause = () => {
		if (playing()) {
			audio.pause();
		} else {
			audio.play();
		}
	};

	const toggleMute = () => setMuted(m => !m);

	const handleVolumeWheel = (e: WheelEvent) => {
		e.preventDefault();
		if (e.deltaY > 0) {
			audio.volume = Math.max(volume() - .05, 0);
		} else {
			audio.volume = Math.min(volume() + .05, 1);
		}
	};

	const handleScrubWheel = (e: WheelEvent) => {
		e.preventDefault();
		if (e.deltaY > 0) {
			audio.currentTime = Math.max(progress() - 5, 0);
		} else {
			audio.currentTime = Math.min(progress() + 5, duration());
		}
	};

	const handleScrubClick = () => {
		const p = progressPreview()!;
		audio.currentTime = p;
		setProgress(p)
	};

	const handleScrubMouseOut = () => {
		setProgressPreview(null);
	};

	const handleScrubMouseMove = (e: MouseEvent) => {
		const target = e.target as HTMLElement;
		const { x, width } = target.getBoundingClientRect();
		const p = ((e.clientX - x) / width) * duration();
		setProgressPreview(p);
		if (e.buttons) {
			audio.currentTime = p;
			setProgress(p)
		}
	};

	const progressWidth = () => `${(progress() / duration()) * 100}%`;
	const progressPreviewWidth = () =>
		progressPreview()
			? `${(progressPreview()! / duration()) * 100}%`
			: undefined;

	const byteFmt = Intl.NumberFormat("en", {
		notation: "compact",
		style: "unit",
		unit: "byte",
		unitDisplay: "narrow",
	});

	const ty = () => props.media.source.mime.split(";")[0];

	const getVolumeIcon = () => {
		if (muted()) return iconVolumeMute;
		if (volume() === 0) return iconVolumeMute;
		if (volume() < .333) return iconVolumeLow;
		if (volume() < .667) return iconVolumeMedium;
		if (volume() <= 1) return iconVolumeHigh;
		return iconVolumeMax;
	};

	const getVolumeText = () => {
		if (muted()) return "muted";
		return `${Math.round(volume() * 100)}%`;
	};

	return (
		<div class="audio">
			<div
				class="progress"
				onWheel={handleScrubWheel}
				onMouseOut={handleScrubMouseOut}
				onMouseMove={handleScrubMouseMove}
				onMouseDown={handleScrubClick}
				onClick={handleScrubClick}
			>
				<div class="fill" style={{ width: progressWidth() }}></div>
				<div class="preview" style={{ width: progressPreviewWidth() }}></div>
			</div>
			<div class="info">
				<a
					download={props.media.filename}
					title={props.media.filename}
					href={props.media.source.url}
				>
					{props.media.filename}
				</a>
				<div class="dim">
					{ty()} - {byteFmt.format(props.media.source.size)}
				</div>
			</div>
			<div class="controls">
				<button onClick={togglePlayPause} title={playing() ? "pause" : "play"}>
					<img
						class="icon"
						src={playing() ? iconPause : iconPlay}
						alt={playing() ? "pause" : "play"}
					/>
				</button>
				{tooltip({
					placement: "top-start",
					interactive: true,
					doesntRetain: "input[type=range]",
				},
					(
						<div class="range">
							<input
								type="range"
								min={0}
								max={1}
								value={volume()}
								disabled={muted()}
								step={.001}
								onInput={e => setVolume(e.target.valueAsNumber)}
							/>
							<div class="dim">(click to mute)</div>
							<div class="value">{getVolumeText()}</div>
						</div>
					) as ValidComponent,
					(<button
						onClick={toggleMute}
						title={getVolumeText()}
						onWheel={handleVolumeWheel}
					>
						<img
							class="icon"
							src={getVolumeIcon()}
							alt={getVolumeText()}
						/>
					</button>
					) as HTMLElement
				)}
				<div class="space"></div>
				<div
					class="time"
					classList={{ preview: progressPreview() !== null }}
					onWheel={handleScrubWheel}
				>
					<span class="progress">
						{formatTime(progressPreview() ?? progress())}
					</span>{" "}
					/ <span class="duration">{formatTime(duration())}</span>
				</div>
			</div>
		</div>
	);
};
