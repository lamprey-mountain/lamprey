import {
	createEffect,
	createSignal,
	onMount,
	ValidComponent,
	VoidProps,
} from "solid-js";
import {
	formatTime,
	getDuration,
	getHeight,
	getWidth,
	MediaProps,
} from "./util.ts";
import iconPlay from "../assets/play.png";
import iconPause from "../assets/pause.png";
import iconVolumeLow from "../assets/volume-low.png";
import iconVolumeMedium from "../assets/volume-medium.png";
import iconVolumeHigh from "../assets/volume-high.png";
import iconVolumeMute from "../assets/volume-mute.png";
import iconVolumeMax from "../assets/volume-max.png";
import { tooltip } from "../Tooltip.tsx";

export const VideoViewOld = (props: MediaProps) => {
	const height = () => getHeight(props.media);
	const width = () => getWidth(props.media);

	return (
		<div
			class="media"
			style={{
				"--height": `${height()}px`,
				"--width": `${width()}px`,
				"--aspect-ratio": `${width()}/${height()}`,
			}}
		>
			<div class="inner">
				<div class="loader">loading</div>
				<video controls src={props.media.source.url} />
			</div>
		</div>
	);
};

export const VideoView = (props: MediaProps) => {
	const height = () => getHeight(props.media);
	const width = () => getWidth(props.media);

	const [duration, setDuration] = createSignal(getDuration(props.media));
	const [progress, setProgress] = createSignal(0);
	const [progressPreview, setProgressPreview] = createSignal<null | number>(
		null,
	);
	const [playing, setPlaying] = createSignal(false);
	const [volume, setVolume] = createSignal(1);
	const [muted, setMuted] = createSignal(false);

	let videoEl!: HTMLVideoElement;
	let wrapperEl!: HTMLDivElement;

	onMount(() => {
		videoEl.ondurationchange = () => setDuration(videoEl.duration);
		videoEl.ontimeupdate = () => setProgress(videoEl.currentTime);
		videoEl.onplay = () => setPlaying(true);
		videoEl.onpause = () => setPlaying(false);
	});

	createEffect(() => videoEl.muted = muted());
	createEffect(() => videoEl.volume = volume());

	const togglePlayPause = () => {
		if (playing()) {
			videoEl.pause();
		} else {
			videoEl.play();
		}
	};

	const toggleMute = () => setMuted((m) => !m);

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
			setVolume(Math.max(volume() - .05, 0));
		} else {
			setVolume(Math.min(volume() + .05, 1));
		}
	};

	const handleScrubClick = () => {
		const p = progressPreview()!;
		videoEl.currentTime = p;
		setProgress(p);
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
			videoEl.currentTime = p;
			setProgress(p);
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
		<div class="video">
			<div
				class="media"
				style={{
					"--height": `${height()}px`,
					"--width": `${width()}px`,
					"--aspect-ratio": `${width()}/${height()}`,
				}}
				ref={wrapperEl!}
			>
				<div class="inner">
					<div class="loader">loading</div>
					<video
						ref={videoEl!}
						src={props.media.source.url}
						onClick={togglePlayPause}
						onDblClick={fullScreen}
					/>
				</div>
			</div>
			<div class="footer">
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
						href={props.media.source.url}
					>
						{props.media.filename}
					</a>
					<div class="dim">
						{ty()} - {byteFmt.format(props.media.source.size)}
					</div>
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
					{tooltip(
						{
							placement: "top-start",
							interactive: true,
							doesntRetain: "input[type=range]",
						},
						(
							<div class="range" onWheel={handleVolumeWheel}>
								<input
									type="range"
									min={0}
									max={1}
									value={volume()}
									disabled={muted()}
									step={.001}
									onInput={(e) => setVolume(e.target.valueAsNumber)}
								/>
								<div class="dim">(click to mute)</div>
								<div class="value">{getVolumeText()}</div>
							</div>
						) as ValidComponent,
						(
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
						) as HTMLElement,
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
		</div>
	);
};
