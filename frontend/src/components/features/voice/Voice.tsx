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
import {
	icCamera,
	icExit,
	icHeadphones,
	icMic,
	icMusic,
	icScreenshare,
} from "@/utils/icons";
import { Icon } from "@/atoms/Icon";
import { ToggleIcon } from "@/atoms/ToggleIcon.tsx";
import { AvatarWithStatus } from "@/components/shared/User";
import { useChannel } from "@/contexts/channel";
import { getColor } from "@/lib/colors";
import { flags } from "@/lib/flags";
import { md } from "@/lib/markdown";
import { useVoice } from "./context.tsx";

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

	// FIXME: this seems to be very janky
	createEffect(
		on(
			() => p.channel.id,
			(tid) => {
				if (!voice.joinedChannelId || voice.joinedChannelId !== tid)
					actions.selectChannel(tid);
			},
		),
	);

	const getUsersWithoutStreams = () => {
		const hasStream = new Set();
		for (const s of voice.vc.streams.values() ?? []) {
			hasStream.add(s.user_id);
		}
		const users = [];
		for (const state of api.voiceStates.values()) {
			if (
				(state as any).thread_id === p.channel.id &&
				!hasStream.has(state.user_id)
			) {
				users.push(state.user_id);
			}
		}
		return users;
	};

	const [focused, setFocused] = createSignal<null | string>(null);
	const [controls, setControls] = createSignal(true);

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
	});

	const isChatOpen = () => ch.voice_chat_sidebar_open;
	const toggleChat = () => {
		chUpdate("voice_chat_sidebar_open", (o) => !o);
	};

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
						<ToggleIcon checked={!voice.deafened} src={icHeadphones} />
					</button>
					<button
						type="button"
						class="button icon-button"
						onClick={() => actions.toggleCamera()}
					>
						<ToggleIcon checked={voice.camera} src={icCamera} />
					</button>
					<button
						type="button"
						class="button icon-button"
						onClick={() => actions.toggleMicrophone()}
					>
						<ToggleIcon checked={!voice.muted} src={icMic} />
					</button>
					<button
						type="button"
						class="button icon-button"
						onClick={actions.toggleScreenshare}
					>
						<ToggleIcon checked={voice.screensharing} src={icScreenshare} />
					</button>
					<Show when={flags.has("voice_music")}>
						<button
							type="button"
							class="button icon-button"
							onClick={actions.playMusic}
						>
							<ToggleIcon checked={voice.musicing} src={icMusic} />
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
