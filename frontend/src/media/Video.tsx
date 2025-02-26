import {
	createEffect,
	createSignal,
	For,
	onCleanup,
	onMount,
	Show,
	ValidComponent,
} from "solid-js";
import {
	byteFmt,
	formatTime,
	getDuration,
	getHeight,
	getUrl,
	getWidth,
	Loader,
	MediaLoadingState,
	MediaProps,
	parseRanges,
	Resize,
} from "./util.tsx";
import iconPlay from "../assets/play.png";
import iconPause from "../assets/pause.png";
import iconVolumeLow from "../assets/volume-low.png";
import iconVolumeMedium from "../assets/volume-medium.png";
import iconVolumeHigh from "../assets/volume-high.png";
import iconVolumeMute from "../assets/volume-mute.png";
import iconVolumeMax from "../assets/volume-max.png";
import iconFullscreen from "../assets/fullscreen.png";
import iconFullscreent from "../assets/fullscreent.png";
import { createTooltip, tooltip } from "../Tooltip.tsx";
import { useCtx } from "../context.ts";

export const VideoView = (props: MediaProps) => {
	const ctx = useCtx();
	const height = () => getHeight(props.media);
	const width = () => getWidth(props.media);

	const [loadingState, setLoadingState] = createSignal<MediaLoadingState>(
		"empty",
	);
	const [buffered, setBuffered] = createSignal(
		[] as ReturnType<typeof parseRanges>,
	);
	const [duration, setDuration] = createSignal(getDuration(props.media));
	const [progress, setProgress] = createSignal(0);
	const [progressPreview, setProgressPreview] = createSignal<null | number>(
		null,
	);
	const [playing, setPlaying] = createSignal(false);
	const [volume, setVolume] = createSignal(1);
	const [muted, setMuted] = createSignal(false);
	const [playbackRate, setPlaybackRate] = createSignal(1);
	const [fullscreen, setFullscreen] = createSignal(false);

	let video!: HTMLVideoElement;
	let wrapperEl!: HTMLDivElement;
	let topEl!: HTMLDivElement;
	// const [wrapperEl]!: HTMLDivElement;

	const volumeTooltip = createTooltip({
		placement: "top-start",
		interactive: true,
		doesntRetain: "input[type=range]",
		mount: () => fullscreen() ? wrapperEl : undefined,
		tip: () => (
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
		),
	});
	const vtc = volumeTooltip.content;

	onMount(() => {
		video.ondurationchange = () => setDuration(video.duration);
		video.ontimeupdate = () => setProgress(video.currentTime);
		video.onratechange = () => setPlaybackRate(video.playbackRate);
		video.onvolumechange = () => setVolume(video.volume);

		video.onplaying = () => {
			const cur = ctx.currentMedia();
			if (cur && cur.media.id !== props.media.id) {
				cur.element.pause();
			}

			ctx.setCurrentMedia({ media: props.media, element: video });
			setHandlers();
			setPlaying(true);
		};

		video.onpause = () => setPlaying(false);
		video.onended = () => setPlaying(false);

		video.onloadedmetadata = () => setLoadingState("ready");
		video.onstalled = () => setLoadingState("stalled");
		video.onsuspend = () => setLoadingState("stalled");
		video.onseeking = () => setLoadingState("loading");
		video.onseeked = () => setLoadingState("ready");
		video.onprogress = () => setBuffered(parseRanges(video.buffered));
		video.oncanplaythrough = () => setBuffered(parseRanges(video.buffered));
		video.onemptied = () => {
			setLoadingState("empty");
			setBuffered(parseRanges(video.buffered));
		};
		video.oncanplay = () => {
			setLoadingState("ready");
			setBuffered(parseRanges(video.buffered));
		};
	});

	createEffect(() => video.muted = muted());
	createEffect(() => video.volume = volume());

	const togglePlayPause = () => {
		if (playing()) {
			video.pause();
		} else {
			video.play();
		}
	};

	const toggleMute = () => setMuted((m) => !m);

	const fullScreenDblClick = (e: MouseEvent) => {
		e.preventDefault();
		e.stopPropagation();
		console.log(e, wrapperEl);
		wrapperEl.requestFullscreen();
		setFullscreen(true);
	};

	const handleScrubWheel = (e: WheelEvent) => {
		e.preventDefault();
		const newt = e.deltaY > 0
			? Math.max(progress() - 5, 0)
			: Math.min(progress() + 5, duration());
		video.currentTime = newt;
		setProgress(newt);
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
		video.currentTime = p;
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
			video.currentTime = p;
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

	const toggleFullscreen = () => {
		if (fullscreen()) {
			document.exitFullscreen();
		} else {
			wrapperEl.requestFullscreen();
		}
	};

	const handleFullscreenChange = () => {
		setFullscreen(document.fullscreenElement === wrapperEl);
		console.log("update");
		volumeTooltip.update();
		requestAnimationFrame(() => {
			volumeTooltip.update();
		});
		setTimeout(() => {
			volumeTooltip.update();
		});
		queueMicrotask(() => {
			volumeTooltip.update();
		});
	};

	const setMetadata = () => {
		navigator.mediaSession.metadata = new MediaMetadata({
			title: props.media.filename,
			// artist: "artist",
			// album: "album",
			artwork: props.media.tracks.filter((i) => i.type === "Thumbnail").map(
				(i) => ({
					src: getUrl(i),
					sizes: `${i.width}x${i.height}`,
					type: i.mime,
				}),
			),
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
		if (playing()) {
			navigator.mediaSession.setPositionState({
				duration: duration(),
				playbackRate: playbackRate(),
				position: progress(),
			});
		}
	});

	onMount(() => {
		wrapperEl.addEventListener("fullscreenchange", handleFullscreenChange);
	});

	onCleanup(() => {
		wrapperEl.removeEventListener("fullscreenchange", handleFullscreenChange);
	});

	return (
		<Resize height={height()} width={width()}>
			<div class="video" ref={wrapperEl!}>
				<Loader loaded={loadingState() !== "empty"} />
				<video
					ref={video!}
					src={getUrl(props.media.source)}
					preload="metadata"
					onClick={togglePlayPause}
					onDblClick={fullScreenDblClick}
				/>
				<div class="footer">
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
					<div class="info">
						<a
							download={props.media.filename}
							title={props.media.filename}
							href={getUrl(props.media.source)}
						>
							{props.media.filename}
						</a>
						<div class="dim">
							{ty()} - {byteFmt.format(props.media.source.size)}
							<Show when={loadingState() === "stalled"}>{" "}- loading</Show>
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
						<button
							onClick={toggleMute}
							title={getVolumeText()}
							onWheel={handleVolumeWheel}
							use:vtc
						>
							<img
								class="icon"
								src={getVolumeIcon()}
								alt={getVolumeText()}
							/>
						</button>
						<button
							onClick={toggleFullscreen}
							title={fullscreen() ? "exit fullscreen" : "enter fullscreen"}
						>
							<img
								class="icon"
								src={fullscreen() ? iconFullscreent : iconFullscreen}
								alt=""
							/>
						</button>
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
				</div>
			</div>
		</Resize>
	);
};
