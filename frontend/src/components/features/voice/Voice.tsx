import type { Channel } from "sdk";
import {
	createEffect,
	createSignal,
	For,
	Match,
	on,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import { useApi } from "@/api";
import { Icon } from "@/atoms/Icon";
import { ToggleIcon } from "@/atoms/ToggleIcon.tsx";
import { AvatarWithStatus } from "@/components/shared/User";
import { useChannel } from "@/contexts/channel";
import { getColor } from "@/lib/colors";
import { flags } from "@/lib/flags";
import { md } from "@/lib/markdown";
import {
	icCamera,
	icExit,
	icHeadphones,
	icMic,
	icMusic,
	icScreenshare,
} from "@/utils/icons";
import { useVoice } from "./context.tsx";
import { Markdown } from "@/atoms/Markdown.tsx";

// TODO:
// - views:
//   - focus: show currently talking person, show list of other people below
//   - grid: a dynamically sized grid of people
//   - fullscreen: make one stream fullscreen, hide everyone else
// - add button to toggle grid/focus view
// - click stream to toggle fullscreen
// - option to hide participants without video
// - option to hide yourself (not your screenshare)

const MemberName = (props: { roomId?: string | null; userId: string }) => {
	const api = useApi();

	const user = api.users.use(() => props.userId);

	const member = api.room_members.use(() =>
		props.roomId ? `${props.roomId}:${props.userId}` : undefined,
	);

	return <>{member()?.override_name || user()?.name || props.userId}</>;
};

export const Voice = (p: { channel: Channel }) => {
	const api = useApi();
	const [voice, actions] = useVoice();
	const [ch, chUpdate] = useChannel()!;


	const getUsersWithoutStreams = () => {
		const hasStream = new Set();
		for (const s of voice.vc.streams.values() ?? []) {
			hasStream.add(s.user_id);
		}
		const users = [];
		for (const state of api.voiceStates.values()) {
			if (
				state.channel_id === p.channel.id &&
				!hasStream.has(state.user_id)
			) {
				users.push(state.user_id);
			}
		}
		return users;
	};

	const [focused, setFocused] = createSignal<null | string>(null);
	const [controls, setControls] = createSignal(true);

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

	// TODO: use @solid-primitives/scheduled
	let controlsTimeout: NodeJS.Timeout = setTimeout(
		() => setControls(false),
		2000,
	);

	const showControls = () => {
		setControls(true);
		clearTimeout(controlsTimeout);
		controlsTimeout = setTimeout(() => setControls(false), 2000);
	};

	const hideControls = () => {
		setControls(false);
		clearTimeout(controlsTimeout);
	};

	onCleanup(() => {
		clearTimeout(controlsTimeout);
		if (prevSubscribed.size > 0) {
			voice.vc.unsubscribeFromTracks([...prevSubscribed]);
		}
	});

	const isChatOpen = () => ch.voice_chat_sidebar_open;
	const toggleChat = () => {
		chUpdate("voice_chat_sidebar_open", (o) => !o);
	};

	// TODO: deduplicate jsx
	// TODO: keep controls/header shown when hovering over controls/header
	// TODO: hide .status and .live when controls are hidden
	// TODO: show muted/deafened indicators in .status
	// TODO: button to open context menu in bottom right corner of each stream
	// TODO: group buttons

	return (
		<div
			class="webrtc"
			classList={{ controls: controls(), "stream-focused": !!focused() }}
			onMouseMove={showControls}
			onMouseOut={hideControls}
		>
			<div class="streams">
				<div class="centered">
					<Show when={focused()}>
						{((stream) => {
							if (!stream) return;
							let videoRef!: HTMLVideoElement;
							createEffect(() => {
								if (videoRef) videoRef.srcObject = stream.media;
							});
							if (
								stream.key === "screen" &&
								!watchedScreenshares().has(stream.id)
							) {
								return (
									<div
										class="stream fullscreen placeholder"
										onClick={() => setFocused(null)}
									>
										<div class="live">live</div>
										<div class="status">
											<MemberName
												userId={stream.user_id}
												roomId={p.channel.room_id}
											/>
											{"'s screen"}
										</div>
										<button
											class="button watch"
											onClick={(e) => {
												e.stopPropagation();
												watchStream(stream.id);
											}}
										>
											Watch
										</button>
									</div>
								);
							}
							return (
								<div
									class="stream"
									classList={{
										fullscreen: focused() === stream.id,
										speaking:
											((voice.vc.speaking.users.get(stream.user_id)?.flags ??
												0) &
												1) ===
											1,
									}}
									onClick={() =>
										setFocused((s) => (s === stream.id ? null : stream.id))
									}
								>
									<div class="live">live</div>
									<video autoplay playsinline ref={videoRef!} muted />
									<div class="status">
										{
											<MemberName
												userId={stream.user_id}
												roomId={p.channel.room_id}
											/>
										}
									</div>
								</div>
							);
						})(voice.vc.streams.get(focused()!))}
					</Show>
					<div class="list">
						<For each={[...(voice.vc.streams.values() ?? [])]}>
							{(stream) => {
								let videoRef!: HTMLVideoElement;
								createEffect(() => {
									if (videoRef) videoRef.srcObject = stream.media;
								});

								if (
									stream.key === "screen" &&
									!watchedScreenshares().has(stream.id)
								) {
									return (
										<div
											class="stream placeholder"
											style={{
												display: focused() === stream.id ? "none" : undefined,
												"flex-direction": "column",
												gap: "8px",
											}}
										>
											<div class="live">live</div>
											<div class="status">
												<MemberName
													userId={stream.user_id}
													roomId={p.channel.room_id}
												/>
												{"'s screen"}
											</div>
											<button
												class="button watch"
												onClick={() => {
													watchStream(stream.id);
													setFocused(stream.id);
												}}
											>
												Watch
											</button>
										</div>
									);
								}

								return (
									<div
										class="stream"
										classList={{
											speaking:
												((voice.vc.speaking.users.get(stream.user_id)?.flags ??
													0) &
													1) ===
												1,
										}}
										style={{
											display: focused() === stream.id ? "none" : undefined,
										}}
										onClick={() =>
											setFocused((s) => (s === stream.id ? null : stream.id))
										}
									>
										<div class="live">live</div>
										<video autoplay playsinline ref={videoRef!} muted />
										<div class="status">
											{
												<MemberName
													userId={stream.user_id}
													roomId={p.channel.room_id}
												/>
											}
										</div>
									</div>
								);
							}}
						</For>
						<For each={getUsersWithoutStreams()}>
							{(uid) => {
								const user = api.users.use(() => uid);
								return (
									<div
										class="stream"
										style={{
											"background-color": getColor(uid),
										}}
									>
										<Show when={user}>
											<AvatarWithStatus user={user()} />
											<div class="status">
												{<MemberName userId={uid} roomId={p.channel.room_id} />}
											</div>
										</Show>
									</div>
								);
							}}
						</For>
					</div>
				</div>
			</div>
			<header class="top">
				<b>{p.channel.name}</b>
				<Show when={p.channel.description}>
					<span class="dim" style="white-space:pre;font-size:1em">
						{"  -  "}
					</span>
					{/* TODO: <Show when={}>{desc => <Markdown content={p.channel.description} />}</Show>*/}
					<span
						class="markdown"
						innerHTML={md(p.channel.description ?? "") as string}
					></span>
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
			<div class="bottom">
				<div class="controls">
					<button
						type="button"
						class="button icon-button"
						onClick={() => actions.toggleDeafened()}
					>
						<ToggleIcon enabled={!voice.deafened} src={icHeadphones} />
					</button>
					<button
						type="button"
						class="button icon-button"
						onClick={() => actions.toggleCamera()}
					>
						<ToggleIcon enabled={voice.camera} src={icCamera} />
					</button>
					<button
						type="button"
						class="button icon-button"
						onClick={() => actions.toggleMicrophone()}
					>
						<ToggleIcon enabled={!voice.muted} src={icMic} />
					</button>
					<button
						type="button"
						class="button icon-button"
						onClick={actions.toggleScreenshare}
					>
						<ToggleIcon enabled={voice.screensharing} src={icScreenshare} />
					</button>
					<Show when={flags.has("voice_music")}>
						<button
							type="button"
							class="button icon-button"
							onClick={actions.playMusic}
						>
							<ToggleIcon enabled={voice.musicing} src={icMusic} />
						</button>
					</Show>
					<Show
						when={
							focused() &&
							voice.vc.streams.get(focused()!)?.key === "screen" &&
							watchedScreenshares().has(focused()!)
						}
					>
						<button
							type="button"
							class="button disconnect icon-button"
							onClick={() => {
								unwatchStream(focused()!);
								setFocused(null);
							}}
						>
							{/* TODO: use a different icon for this */}
							{/* TODO: add a tooltip "Stop Watching" */}
							<Icon src={icExit} />
						</button>
					</Show>
					<button
						type="button"
						class="button disconnect icon-button"
						onClick={actions.disconnect}
					>
						<Icon src={icExit} />
					</button>
				</div>
			</div>
		</div>
	);
};
