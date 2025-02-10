import { createSignal, onMount, VoidProps } from "solid-js";
import { Media } from "sdk";
import { formatTime } from "./util.ts";
import iconPlay from "../assets/play.png";
import iconPause from "../assets/pause.png";
import iconVolumeLow from "../assets/volume-low.png";
import iconVolumeMedium from "../assets/volume-medium.png";
import iconVolumeHigh from "../assets/volume-high.png";
import iconVolumeMute from "../assets/volume-mute.png";
import iconVolumeMax from "../assets/volume-max.png";

type MediaProps = VoidProps<{ media: Media }>;

export const VideoViewOld = (props: MediaProps) => {
	return (
		<div
			class="media"
			style={{
				"--height": `${props.media.height}px`,
				"--width": `${props.media.width}px`,
				"--aspect-ratio": `${props.media.width}/${props.media.height}`,
			}}
		>
			<div class="inner">
				<div class="loader">loading</div>
				<video controls src={props.media.url} />
			</div>
		</div>
	);
};

export const VideoView = (props: MediaProps) => {
	const [duration, setDuration] = createSignal(
		(props.media.duration ?? 0) / 1000,
	);
	const [progress, setProgress] = createSignal(0);
	const [progressPreview, setProgressPreview] = createSignal<null | number>(
		null,
	);
	const [playing, setPlaying] = createSignal(false);
	const [volume, setVolume] = createSignal(1);
	const [muted, setMuted] = createSignal(false);

	let videoEl: HTMLVideoElement;
	let wrapperEl: HTMLDivElement;

	onMount(() => {
		videoEl.ondurationchange = () => setDuration(videoEl.duration);
		videoEl.ontimeupdate = () => setProgress(videoEl.currentTime);
		videoEl.onplay = () => setPlaying(true);
		videoEl.onpause = () => setPlaying(false);
		videoEl.onvolumechange = () => setVolume(videoEl.volume);
	});

	const togglePlayPause = () => {
		if (playing()) {
			videoEl.pause();
		} else {
			videoEl.play();
		}
	};

	const toggleMute = () => {
		if (muted()) {
			setMuted(false);
		} else {
			setMuted(true);
		}
	};

	const fullScreen = (e: MouseEvent) => {
		e.preventDefault();
		e.stopPropagation();
		console.log(e, wrapperEl);
		wrapperEl.requestFullscreen();
	};

	const handleScrubWheel = (e: WheelEvent) => {
		e.preventDefault();
		if (e.deltaY > 0) {
			videoEl.currentTime = Math.max(progress() - 5, 0);
		} else {
			videoEl.currentTime = Math.min(progress() + 5, duration());
		}
	};

	const handleVolumeWheel = (e: WheelEvent) => {
		e.preventDefault();
		if (e.deltaY > 0) {
			videoEl.volume = Math.max(volume() - .05, 0);
		} else {
			videoEl.volume = Math.min(volume() + .05, 1);
		}
	};

	const handleScrubClick = () => {
		videoEl.currentTime = progressPreview()!;
	};

	const handleScrubMouseOut = () => {
		setProgressPreview(null);
	};

	const handleScrubMouseMove = (e: MouseEvent) => {
		const target = e.target as HTMLElement;
		const { x, width } = target.getBoundingClientRect();
		const p = ((e.clientX - x) / width) * duration();
		setProgressPreview(p);
		if (e.buttons) videoEl.currentTime = p;
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

	const ty = () => props.media.mime.split(";")[0];

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
		<div
			class="media video"
			style={{
				"--height": `${props.media.height}px`,
				"--width": `${props.media.width}px`,
				"--aspect-ratio": `${props.media.width}/${props.media.height}`,
			}}
			ref={wrapperEl!}
		>
			<div class="inner">
				<div class="loader">loading</div>
				<video
					ref={videoEl!}
					src={props.media.url}
					onClick={togglePlayPause}
					onDblClick={fullScreen}
				/>
			</div>
			<div class="overlay">
				<div
					class="progress"
					onWheel={handleScrubWheel}
					onMouseOut={handleScrubMouseOut}
					onMouseMove={handleScrubMouseMove}
					onMouseDown={handleScrubClick}
					onClick={handleScrubClick}
				>
					<div class="fill" style={{ width: progressWidth() }}></div>
					<div class="preview" style={{ width: progressPreviewWidth() }}>
					</div>
				</div>
				<div class="info">
					<a
						download={props.media.filename}
						title={props.media.filename}
						href={props.media.url}
					>
						{props.media.filename}
					</a>
					<div class="dim">{ty()} - {byteFmt.format(props.media.size)}</div>
				</div>
				<div class="controls">
					<button
						onClick={togglePlayPause}
						title={playing() ? "pause" : "play"}
					>
						<img
							class="icon"
							src={playing() ? iconPause : iconPlay}
							alt={playing() ? "pause" : "play"}
						/>
					</button>
					<button
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
		</div>
	);
};
