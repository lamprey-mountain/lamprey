import {
	createEffect,
	createSignal,
	createUniqueId,
	For,
	onCleanup,
	Show,
	type ValidComponent,
} from "solid-js";
import iconPlay from "../assets/play.png";
import iconPause from "../assets/pause.png";
import iconVolumeLow from "../assets/volume-low.png";
import iconVolumeMedium from "../assets/volume-medium.png";
import iconVolumeHigh from "../assets/volume-high.png";
import iconVolumeMute from "../assets/volume-mute.png";
import iconVolumeMax from "../assets/volume-max.png";
import {
	byteFmt,
	formatTime,
	getDuration,
	getThumb,
	getUrl,
	type MediaLoadingState,
	type MediaProps,
	parseRanges,
} from "./util.tsx";
import { tooltip } from "../Tooltip.tsx";
import { useCtx } from "../context.ts";

export const AudioView = (props: MediaProps) => {
	const ctx = useCtx();

	const audio = new Audio();
	const actx = new AudioContext();
	const gain = actx.createGain();
	const source = actx.createMediaElementSource(audio);
	source.connect(gain);
	gain.connect(actx.destination);

	audio.preload = "metadata";
	audio.crossOrigin = "anonymous";
	createEffect(() => audio.src = getUrl(props.media));
	onCleanup(() => audio.pause());

	const [loadingState, setLoadingState] = createSignal<MediaLoadingState>(
		"empty",
	);
	const [buffered, setBuffered] = createSignal(parseRanges(audio.buffered));
	const [duration, setDuration] = createSignal(getDuration(props.media));
	const [progress, setProgress] = createSignal(0);
	const [progressPreview, setProgressPreview] = createSignal<null | number>(
		null,
	);
	const [playing, setPlaying] = createSignal(false);
	const [volume, setVolume] = createSignal(1);
	const [muted, setMuted] = createSignal(false);
	const [playbackRate, setPlaybackRate] = createSignal(1);

	audio.ondurationchange = () => setDuration(audio.duration);
	audio.ontimeupdate = () => setProgress(audio.currentTime);
	audio.onratechange = () => setPlaybackRate(audio.playbackRate);
	audio.onplay = () => setPlaying(true);

	audio.onplaying = () => {
		const cur = ctx.currentMedia();
		if (cur && cur.media.id !== props.media.id) {
			cur.element.pause();
		}

		actx.resume();
		ctx.setCurrentMedia({ media: props.media, element: audio });
		setHandlers();
		setPlaying(true);
	};

	audio.onpause = () => setPlaying(false);
	audio.onended = () => setPlaying(false);

	audio.onloadedmetadata = () => setLoadingState("ready");
	audio.onstalled = () => setLoadingState("stalled");
	audio.onseeking = () => setLoadingState("stalled");
	audio.onseeked = () => setLoadingState("ready");
	audio.onprogress = () => setBuffered(parseRanges(audio.buffered));
	audio.oncanplaythrough = () => setBuffered(parseRanges(audio.buffered));
	audio.onemptied = () => {
		setLoadingState("empty");
		setBuffered(parseRanges(audio.buffered));
	};
	audio.oncanplay = () => {
		setLoadingState("ready");
		setBuffered(parseRanges(audio.buffered));
	};

	createEffect(() => audio.muted = muted());
	createEffect(() => gain.gain.value = volume());

	const volumeDatalistId = createUniqueId();

	const togglePlayPause = () => {
		if (audio.paused) {
			audio.play();
		} else {
			audio.pause();
		}
	};

	const toggleMute = () => setMuted((m) => !m);

	const handleVolumeWheel = (e: WheelEvent) => {
		e.preventDefault();
		if (e.deltaY > 0) {
			setVolume(Math.max(volume() - .05, 0));
		} else {
			setVolume(Math.min(volume() + .05, 1));
		}
	};

	const handleScrubWheel = (e: WheelEvent) => {
		e.preventDefault();
		const newt = e.deltaY > 0
			? Math.max(progress() - 5, 0)
			: Math.min(progress() + 5, duration());
		audio.currentTime = newt;
		setProgress(newt);
	};

	const handleScrubClick = () => {
		const p = progressPreview()!;
		audio.currentTime = p;
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
			audio.currentTime = p;
			setProgress(p);
		}
	};

	const progressWidth = () => `${(progress() / duration()) * 100}%`;
	const progressPreviewWidth = () =>
		progressPreview()
			? `${(progressPreview()! / duration()) * 100}%`
			: undefined;

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

	const setMetadata = () => {
		navigator.mediaSession.metadata = new MediaMetadata({
			title: props.media.filename,
			// artist: "artist",
			// album: "album",
			artwork: [{
				sizes: "640x640",
				src: getThumb(props.media, 640),
				type: "image/avif"
			}],
		});
	};

	const setHandlers = () => {
		// navigator.mediaSession.setActionHandler("nexttrack", () => { });
		// navigator.mediaSession.setActionHandler("previoustrack", () => { });
	};

	createEffect(() => {
		if (playing()) setMetadata();
	});

	createEffect(() => {
		if (playing() && progress() < duration()) {
			navigator.mediaSession.setPositionState({
				duration: duration(),
				playbackRate: playbackRate(),
				position: progress(),
			});
		}
	});

	return (
		<article class="audio">
			<svg
				class="progress"
				viewBox="0 0 1 1"
				preserveAspectRatio="none"
				onWheel={handleScrubWheel}
				onMouseOut={handleScrubMouseOut}
				onMouseMove={handleScrubMouseMove}
				onMouseDown={handleScrubClick}
				onClick={handleScrubClick}
				role="progressbar"
				aria-valuemax={duration()}
				aria-valuenow={progress()}
				aria-label="progress"
			>
				<For each={buffered()}>
					{(r) => {
						return (
							<rect
								class="loaded"
								x={r.start / duration()}
								width={(r.end - r.start) / duration()}
							/>
						);
					}}
				</For>
				<rect class="current" width={progressWidth()} />
				<rect class="preview" width={progressPreviewWidth()} fill="#fff3" />
			</svg>
			<a class="thumb" href={getThumb(props.media)} >
				<img src={getThumb(props.media, 64)}
					onError={(e) => {
						console.error("failed to load image");
						e.currentTarget.src = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw=="
					}} />
			</a>
			<div class="info">
				<a
					download={props.media.filename}
					title={props.media.filename}
					href={getUrl(props.media)}
				>
					{props.media.filename}
				</a>
				<div class="dim">
					{ty()} - {byteFmt.format(props.media.source.size)}
					<Show when={loadingState() === "stalled"}>{" "}- loading</Show>
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
								max={1.5}
								value={volume()}
								disabled={muted()}
								step={.001}
								list={volumeDatalistId}
								onInput={(e) => setVolume(e.target.valueAsNumber)}
							/>
							<datalist id={volumeDatalistId}>
								<option value="1" label="100%"></option>
							</datalist>
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
					role="timer"
					aria-label="position"
				>
					<span class="progress">
						{formatTime(progressPreview() ?? progress())}
					</span>{" "}
					/ <span class="duration">{formatTime(duration())}</span>
				</div>
			</div>
		</article>
	);
};
