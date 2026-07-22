import { debounce } from "@solid-primitives/scheduled";
import type { Channel } from "sdk";
import {
	createEffect,
	createSignal,
	For,
	Match,
	on,
	onCleanup,
	onMount,
	Show,
	Switch,
} from "solid-js";
import { useApi } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown.tsx";
import { ToggleIcon } from "@/atoms/ToggleIcon.tsx";
import { createTooltip } from "@/atoms/Tooltip";
import { Avatar, AvatarWithStatus } from "@/components/shared/User";
import { useChannel } from "@/contexts/channel";
import { useMenu } from "@/contexts/menu.tsx";
import { getColor } from "@/lib/colors";
import { flags } from "@/lib/flags";
import {
	icCamera,
	icExit,
	icHeadphones,
	icMic,
	icMore,
	icMusic,
	icScreenshare,
} from "@/utils/icons";
import { useVoice } from "./context.tsx";

// TODO:
// - views:
//   - focus: show currently talking person, show list of other people below
//   - grid: a dynamically sized grid of people
//   - fullscreen: make one stream fullscreen, hide everyone else
// - add button to toggle grid/focus view
// - click stream to toggle fullscreen
// - option to hide participants without video
// - option to hide yourself (not your screenshare)

// FIXME: if self_video becomes false, hide the user stream entirely

export const Voice = (p: { channel: Channel }) => {
	const api = useApi();
	const [voice, actions] = useVoice();
	const [ch, chUpdate] = useChannel()!;
	const { setMenu } = useMenu();
	const [fullscreenStream, setFullscreenStream] = createSignal<null | string>(
		null,
	);
	const [uiVisible, setUiVisible] = createSignal(true);

	const deafenedTooltip = createTooltip({
		tip: () => (voice.deafened ? "Undeafen" : "Deafen"),
	});
	const cameraTooltip = createTooltip({
		tip: () => (voice.camera ? "Disable camera" : "Enable camera"),
	});
	const micTooltip = createTooltip({
		tip: () => (voice.muted ? "Unmute" : "Mute"),
	});
	const screenshareTooltip = createTooltip({
		tip: () => (voice.screensharing ? "Stop Screenshare" : "Start Screenshare"),
	});
	const musicTooltip = createTooltip({
		tip: () => (voice.musicing ? "Stop Music" : "Play Music"),
	});
	const stopWatchingTooltip = createTooltip({
		tip: () => "Stop Watching",
	});
	const disconnectTooltip = createTooltip({
		tip: () => "Disconnect",
	});

	const getUsersWithoutStreams = () => {
		const hasStream = new Set();
		for (const s of voice.vc.streams.values() ?? []) {
			hasStream.add(s.user_id);
		}
		const users = [];
		for (const state of api.voiceStates.values()) {
			if (state.channel_id === p.channel.id && !hasStream.has(state.user_id)) {
				users.push(state.user_id);
			}
		}
		return users;
	};

	// TODO: use ReactiveSet
	const [watchedScreenshares, setWatchedScreenshares] = createSignal<
		Set<string>
	>(new Set());

	const watchStream = (streamId: string) => {
		setWatchedScreenshares((s) => new Set([...s, streamId]));
	};

	const unwatchStream = (streamId: string) => {
		setWatchedScreenshares(
			(s) => new Set([...s].filter((id) => id !== streamId)),
		);
	};

	// TODO: use on(() => subscribed, (prev) => ...)?
	// TODO: resubscribe when reopening channel
	let prevSubscribed = new Set<string>();
	createEffect(() => {
		const allTracks = Array.from(voice.vc.tracks.values());
		const userTracks = allTracks.filter(
			(t) => t.metadata.key === "user" && t.id,
		);
		const watched = watchedScreenshares();
		const screenTracks = allTracks.filter(
			(t) =>
				t.metadata.key === "screen" &&
				t.id &&
				watched.has(`${t.user_id}:screen`),
		);
		const trackIds = [...userTracks, ...screenTracks].map(
			(t) => t.id as string,
		);

		const toAdd = trackIds.filter((id) => !prevSubscribed.has(id));
		const toRemove = [...prevSubscribed].filter((id) => !trackIds.includes(id));

		if (toAdd.length > 0) voice.vc.subscribeToTracks(toAdd);
		if (toRemove.length > 0) voice.vc.unsubscribeFromTracks(toRemove);

		prevSubscribed = new Set(trackIds);
	});

	const hideUi = debounce(() => {
		setUiVisible(false);
	}, 1500);

	const resetUiTimeout = () => {
		setUiVisible(true);
		hideUi();
	};

	const hideUiImmediately = () => {
		hideUi.clear();
		setUiVisible(false);
	};

	onCleanup(() => {
		hideUi.clear(); // NOTE: is this necessary?
		if (prevSubscribed.size > 0) {
			voice.vc.unsubscribeFromTracks([...prevSubscribed]);
		}
	});

	onMount(() => {
		hideUi();
	});

	const isChatOpen = () => ch.voice_chat_sidebar_open;
	const toggleChat = () => {
		chUpdate("voice_chat_sidebar_open", (o) => !o);
	};

	const gridView = () =>
		((p.channel.preferences?.frontend as any)?.grid_view as boolean) ?? false;

	const autoFocusedStreamId = () => {
		const watched = [...watchedScreenshares()];
		if (watched.length > 0) return watched[0];
		const streams = [...(voice.vc.streams.values() ?? [])];
		if (streams.length > 0) return streams[0].id;
		return null;
	};

	const focusedStreamId = () => fullscreenStream() || autoFocusedStreamId();

	const layoutMode = () =>
		fullscreenStream() ? "fullscreen" : gridView() ? "grid" : "focus";

	// TODO: keep controls/header shown when hovering over controls/header
	// TODO: show muted/deafened indicators in .status (always show, even with .hide-ui)
	// TODO: button to open context menu in bottom right corner of each stream
	// TODO: group buttons in .controls

	return (
		<div
			class="voice"
			classList={{
				"hide-ui": !uiVisible(),
			}}
			onMouseMove={resetUiTimeout}
			onMouseOut={hideUiImmediately}
		>
			<div class="streams" data-layout={layoutMode()}>
				<div class="streams-inner">
					<Show
						when={layoutMode() === "focus" || layoutMode() === "fullscreen"}
					>
						<Show when={focusedStreamId()}>
							{(streamId) => (
								<Show when={voice.vc.streams.get(streamId())}>
									{(s) => (
										<div class="focused">
											<Stream
												stream={{ ...s(), vc: voice.vc }}
												subscribed={watchedScreenshares().has(s().id)}
												channelId={p.channel.id}
												onWatch={(id) => watchStream(id)}
												onClick={() =>
													setFullscreenStream((s) =>
														s === streamId() ? null : streamId(),
													)
												}
											/>
										</div>
									)}
								</Show>
							)}
						</Show>
					</Show>
					<Show when={layoutMode() !== "fullscreen"}>
						<div class="list">
							<For
								each={[...(voice.vc.streams.values() ?? [])].filter(
									(s) => layoutMode() === "grid" || s.id !== focusedStreamId(),
								)}
							>
								{(stream) => (
									<Stream
										stream={{ ...stream, vc: voice.vc }}
										subscribed={watchedScreenshares().has(stream.id)}
										channelId={p.channel.id}
										onWatch={(id) => watchStream(id)}
										onClick={() =>
											setFullscreenStream((s) =>
												s === stream.id ? null : stream.id,
											)
										}
									/>
								)}
							</For>
							<For
								each={getUsersWithoutStreams().filter(
									(uid) => layoutMode() === "grid" || uid !== focusedStreamId(),
								)}
							>
								{(uid) => {
									// TODO: merge this with Stream
									const user = api.users.use(() => uid);
									return (
										<div
											class="stream"
											style={{
												"background-color": getColor(uid),
											}}
										>
											<Show when={user()}>
												{(user) => (
													<>
														<Avatar user={user()} />
														<div class="status">
															{
																<MemberName
																	userId={uid}
																	roomId={p.channel.room_id}
																/>
															}
														</div>
													</>
												)}
											</Show>
										</div>
									);
								}}
							</For>
						</div>
					</Show>
				</div>
			</div>
			<header class="header">
				{/* TODO: copy ChatHeader click to edit channel name (among other things) */}
				<b>{p.channel.name}</b>
				<Show when={p.channel.description}>
					{(desc) => (
						<>
							<span class="dim" style="white-space:pre;font-size:1em">
								{"  -  "}
							</span>
							<Markdown
								content={desc()}
								channel_id={p.channel.id}
								inline={true}
								class="markdown"
							/>
						</>
					)}
				</Show>
				<Switch>
					<Match when={p.channel.deleted_at}>{" (removed)"}</Match>
					<Match when={p.channel.archived_at}>{" (archived)"}</Match>
				</Switch>
				<div style="flex:1"></div>
				<button
					type="button"
					class="button"
					onClick={toggleChat}
					classList={{ active: isChatOpen() }}
					title="Show chat"
				>
					Chat
				</button>
			</header>
			<div class="footer">
				<div class="controls">
					<button
						type="button"
						class="button icon-button"
						ref={deafenedTooltip.content}
						onClick={() => actions.toggleDeafened()}
					>
						<ToggleIcon enabled={!voice.deafened} src={icHeadphones} />
					</button>
					<button
						type="button"
						class="button icon-button"
						ref={cameraTooltip.content}
						onClick={() => actions.toggleCamera()}
					>
						<ToggleIcon enabled={voice.camera} src={icCamera} />
					</button>
					<button
						type="button"
						class="button icon-button"
						ref={micTooltip.content}
						onClick={() => actions.toggleMicrophone()}
					>
						<ToggleIcon enabled={!voice.muted} src={icMic} />
					</button>
					<button
						type="button"
						class="button icon-button"
						ref={screenshareTooltip.content}
						onClick={actions.toggleScreenshare}
					>
						<ToggleIcon enabled={voice.screensharing} src={icScreenshare} />
					</button>
					<Show when={flags.has("voice_music")}>
						<button
							type="button"
							class="button icon-button"
							ref={musicTooltip.content}
							onClick={actions.playMusic}
						>
							<ToggleIcon enabled={voice.musicing} src={icMusic} />
						</button>
					</Show>
					<Show
						when={
							focusedStreamId() &&
							voice.vc.streams.get(focusedStreamId()!)?.key === "screen" &&
							watchedScreenshares().has(focusedStreamId()!)
						}
					>
						<button
							type="button"
							class="button disconnect icon-button"
							ref={stopWatchingTooltip.content}
							onClick={() => {
								unwatchStream(focusedStreamId()!);
								setFullscreenStream(null);
							}}
						>
							{/* TODO: use a different icon for this */}
							<Icon src={icExit} />
						</button>
					</Show>
					<button
						type="button"
						class="button icon-button"
						onClick={(e) => {
							e.stopPropagation();
							// TODO: open the context menu above the menu button
							setMenu({
								type: "voice",
								channel_id: p.channel.id,
								x: e.clientX,
								y: e.clientY,
							});
						}}
					>
						<Icon src={icMore} />
					</button>
					<button
						type="button"
						class="button disconnect icon-button"
						ref={disconnectTooltip.content}
						onClick={actions.disconnect}
					>
						<Icon src={icExit} />
					</button>
				</div>
			</div>
		</div>
	);
};

const MemberName = (props: { roomId?: string | null; userId: string }) => {
	const api = useApi();

	const user = api.users.use(() => props.userId);

	const member = api.room_members.use(() =>
		props.roomId ? `${props.roomId}:${props.userId}` : undefined,
	);

	return <>{member()?.override_name || user()?.name || props.userId}</>;
};

const Stream = (props: {
	stream: any; // TODO: better types
	subscribed: boolean;

	// TODO: move below into a solidjs context?
	channelId: string;
	roomId: string;
	onWatch: (id: string) => void;
	onClick: () => void;
}) => {
	let videoRef!: HTMLVideoElement;

	createEffect(() => {
		if (videoRef && props.stream.media) videoRef.srcObject = props.stream.media;
	});

	const speaking = () =>
		((props.stream.vc.speaking.users.get(props.stream.user_id)?.flags ?? 0) &
			1) ===
		1;

	const screenshare = () => props.stream.key === "screen";
	const placeholder = () => screenshare() && !props.subscribed;

	return (
		<div
			class="stream"
			classList={{
				placeholder: placeholder(),
				speaking: speaking(),
			}}
			onClick={props.onClick}
		>
			<Show when={screenshare()}>
				<div class="live">live</div>
			</Show>
			<div class="status">
				<MemberName userId={props.stream.user_id} roomId={props.roomId} />
				<Show when={screenshare()}>{"'s screen"}</Show>
			</div>
			<Show
				when={placeholder()}
				fallback={<video autoplay playsinline ref={videoRef!} muted />}
			>
				<button
					class="button watch"
					onClick={(e) => {
						e.stopPropagation();
						props.onWatch(props.stream.id);
					}}
				>
					Watch
				</button>
			</Show>
		</div>
	);
};
