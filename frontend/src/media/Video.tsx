import { debounce } from "@solid-primitives/scheduled";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import { useCtx } from "@/app/context";
import { Icon } from "@/atoms/Icon";
import { createTooltip } from "@/atoms/Tooltip.tsx";
import {
	icFullscreen,
	icFullscreent,
	icPause,
	icPlay,
	icVolumeHigh,
	icVolumeLow,
	icVolumeMax,
	icVolumeMedium,
	icVolumeMute,
} from "@/utils/icons";
import {
	formatBytes,
	formatTime,
	getDuration,
	getHeight,
	getThumb,
	getUrl,
	getWidth,
	Loader,
	type MediaLoadingState,
	type MediaProps,
	parseRanges,
	Resize,
} from "./util.tsx";

export const VideoView = (props: MediaProps) => {
	const ctx = useCtx();
	const height = () => getHeight(props.media);
	const width = () => getWidth(props.media);

	const [loadingState, setLoadingState] =
		createSignal<MediaLoadingState>("empty");
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
	const [uiVisible, setUiVisible] = createSignal(true);

	let video!: HTMLVideoElement;
	let wrapperEl!: HTMLDivElement;

	const hideUi = debounce(() => {
		if (playing()) {
			setUiVisible(false);
		}
	}, 1500);

	const resetHideUiTimer = () => {
		setUiVisible(true);
		hideUi();
	};

	const keepVisible = (e: Event) => {
		e.stopPropagation();
		hideUi.clear();
		setUiVisible(true);
	};

	createEffect(() => {
		if (!playing()) {
			setUiVisible(true);
			hideUi.clear();
		} else {
			resetHideUiTimer();
		}
	});

	const volumeTooltip = createTooltip({
		placement: "top-start",
		interactive: true,
		doesntRetain: "input[type=range]",
		altBoundary: true,
		mount: () => (fullscreen() ? wrapperEl : undefined),
		tip: () => (
			<div class="range" onWheel={handleVolumeWheel}>
				<input
					type="range"
					min={0}
					max={1}
					value={volume()}
					disabled={muted()}
					step={0.001}
					onInput={(e) => setVolume(e.target.valueAsNumber)}
				/>
				<div class="dim">(click to mute)</div>
				<div class="value">{getVolumeText()}</div>
			</div>
		),
	});

	function createOverlayFlash(el: () => HTMLElement) {
		let fadeAnim: Animation | undefined;

		const startFadeOut = debounce(() => {
			fadeAnim = el().animate([{ opacity: 1 }, { opacity: 0 }], {
				duration: 200,
				fill: "forwards",
			});
		}, 500);

		return {
			trigger: () => {
				fadeAnim?.cancel();
				el().style.opacity = "1";
				startFadeOut();
			},
			cancel: () => {
				fadeAnim?.cancel();
				el().style.opacity = "1";
			},
		};
	}

	let overlayPlayPauseEl!: HTMLDivElement;
	let overlayVolumeEl!: HTMLDivElement;

	const triggerVolume = createOverlayFlash(() => overlayVolumeEl);
	const triggerPlayPause = createOverlayFlash(() => overlayPlayPauseEl);

	createEffect(() => (video.muted = muted()));
	createEffect(() => (video.volume = volume()));

	const togglePlayPause = () => {
		if (playing()) {
			video.pause();
		} else {
			video.play();
		}
		triggerPlayPause.trigger();
	};

	const toggleMute = () => {
		setMuted((m) => !m);
		triggerVolume.trigger();
	};

	const fullScreenDblClick = (e: MouseEvent) => {
		e.preventDefault();
		e.stopPropagation();
		if (fullscreen()) {
			document.exitFullscreen();
			setFullscreen(false);
		} else {
			wrapperEl.requestFullscreen();
			setFullscreen(true);
		}
	};

	const handleScrubWheel = (e: WheelEvent) => {
		e.preventDefault();
		const newt =
			e.deltaY > 0
				? Math.max(progress() - 5, 0)
				: Math.min(progress() + 5, duration());
		video.currentTime = newt;
		setProgress(newt);
	};

	const handleVolumeWheel = (e: WheelEvent) => {
		e.preventDefault();
		if (e.deltaY > 0) {
			setVolume(Math.max(volume() - 0.05, 0));
		} else {
			setVolume(Math.min(volume() + 0.05, 1));
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

	const ty = createMemo(() => props.media.content_type.split(";")[0]);
	const showAlways = createMemo(
		() => !playing() && (progress() === 0 || progress() >= duration()),
	);

	const getVolumeIcon = () => {
		if (muted()) return icVolumeMute;
		if (volume() === 0) return icVolumeMute;
		if (volume() < 0.333) return icVolumeLow;
		if (volume() < 0.667) return icVolumeMedium;
		if (volume() <= 1) return icVolumeHigh;
		return icVolumeMax;
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
		// PERF: clean this up
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
			artwork: [
				{
					sizes: "640x640",
					src: getThumb(props.media, 640),
					type: "image/avif",
				},
			],
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

	const handleKeydown = (e: KeyboardEvent) => {
		switch (e.code) {
			case "ArrowLeft": {
				e.preventDefault();
				video.currentTime = Math.max(video.currentTime - 5, 0);
				break;
			}
			case "ArrowRight": {
				e.preventDefault();
				video.currentTime = Math.min(video.currentTime + 5, duration());
				break;
			}
			case "ArrowUp": {
				e.preventDefault();
				setVolume(Math.min(volume() + 0.05, 1));
				triggerVolume.trigger();
				break;
			}
			case "ArrowDown": {
				e.preventDefault();
				setVolume(Math.max(volume() - 0.05, 0));
				triggerVolume.trigger();
				break;
			}
			case "Space": {
				e.preventDefault();
				togglePlayPause();
				break;
			}
			case "KeyF": {
				toggleFullscreen();
				break;
			}
			case "KeyM": {
				toggleMute();
				break;
			}
			case "Comma": {
				video.currentTime = Math.max(video.currentTime - 0.05, 0);
				break;
			}
			case "Period": {
				video.currentTime = Math.min(video.currentTime + 0.05, duration());
				break;
			}
			case "Home": {
				e.preventDefault();
				video.currentTime = 0;
				break;
			}
			case "End": {
				e.preventDefault();
				video.currentTime = duration();
				break;
			}
		}
	};

	createEffect(() => {
		if (showAlways()) triggerPlayPause.cancel();
	});

	return (
		<Resize height={height()} width={width()}>
			{/* TODO: use <article></article> */}
			<div
				class="video"
				classList={{ "hide-ui": !uiVisible() }}
				ref={wrapperEl!}
				onKeyDown={handleKeydown}
				onMouseMove={resetHideUiTimer}
				tabIndex={0}
			>
				<Loader loaded={loadingState() !== "empty"} />
				<header class="header" onMouseMove={keepVisible}>
					<div class="info">
						<a
							download={props.media.filename}
							title={props.media.filename}
							href={getUrl(props.media)}
							// TODO: tooltip "download"
						>
							{props.media.filename}
						</a>
						<div class="dim">
							{ty()} - {formatBytes(props.media.size)}
							<Show when={loadingState() === "stalled"}> - loading</Show>
						</div>
					</div>
				</header>
				<video
					ref={video!}
					src={getUrl(props.media)}
					preload="metadata"
					onClick={togglePlayPause}
					onDblClick={fullScreenDblClick}
					onDurationChange={() => setDuration(video.duration)}
					onTimeUpdate={() => setProgress(video.currentTime)}
					onRateChange={() => setPlaybackRate(video.playbackRate)}
					onVolumeChange={() => setVolume(video.volume)}
					onPlaying={() => {
						const cur = ctx.currentMedia();
						if (cur && cur.media.id !== props.media.id) {
							cur.element.pause();
						}

						ctx.setCurrentMedia({ media: props.media, element: video });
						setHandlers();
						setPlaying(true);
					}}
					onPause={() => setPlaying(false)}
					onEnded={() => setPlaying(false)}
					onLoadedMetadata={() => setLoadingState("ready")}
					onStalled={() => setLoadingState("stalled")}
					onSuspend={() => setLoadingState("stalled")}
					onSeeking={() => setLoadingState("loading")}
					onSeeked={() => setLoadingState("ready")}
					onProgress={() => setBuffered(parseRanges(video.buffered))}
					onCanPlayThrough={() => setBuffered(parseRanges(video.buffered))}
					onEmptied={() => {
						setLoadingState("empty");
						setBuffered(parseRanges(video.buffered));
					}}
					onCanPlay={() => {
						setLoadingState("ready");
						setBuffered(parseRanges(video.buffered));
					}}
				/>
				{/* TODO: use <footer></footer> */}
				<div class="footer" onMouseMove={keepVisible}>
					<svg
						aria-hidden="true"
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
					<div class="controls">
						{/* TODO: use <menu></menu> */}
						<button
							type="button"
							class="button icon-button"
							onClick={togglePlayPause}
							title={playing() ? "pause" : "play"}
						>
							<Icon
								src={playing() ? icPause : icPlay}
								alt={playing() ? "pause" : "play"}
							/>
						</button>
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
						<div class="spacer"></div>
						<button
							type="button"
							class="button icon-button"
							onClick={toggleMute}
							title={getVolumeText()}
							onWheel={handleVolumeWheel}
							ref={volumeTooltip.content}
						>
							<Icon src={getVolumeIcon()} alt={getVolumeText()} />
						</button>
						<button
							type="button"
							class="button icon-button"
							onClick={toggleFullscreen}
							title={fullscreen() ? "exit fullscreen" : "enter fullscreen"}
						>
							<Icon src={fullscreen() ? icFullscreent : icFullscreen} alt="" />
						</button>
					</div>
				</div>
				<div class="overlay">
					<div class="layer volume" ref={overlayVolumeEl!}>
						<Icon src={getVolumeIcon()} alt="" />
						{muted() ? "muted" : `${Math.round(volume() * 100)}%`}
					</div>
					<div class="layer play-pause" ref={overlayPlayPauseEl!}>
						<Icon
							src={playing() ? icPause : icPlay}
							alt={playing() ? "pause" : "play"}
						/>
					</div>
				</div>
			</div>
		</Resize>
	);
};
