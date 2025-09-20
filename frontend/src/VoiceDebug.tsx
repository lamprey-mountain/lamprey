import {
	createMemo,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import { getAttributeDescription, parseSessionDescription } from "./rtc-util";
import { useVoice } from "./voice-provider";

export const VoiceDebug = (props: { onClose: () => void }) => {
	const [voice] = useVoice();

	const [tab, setTab] = createSignal("sdp-local");
	const [localSdp, setLocalSdp] = createSignal<string | null>(null);
	const [remoteSdp, setRemoteSdp] = createSignal<string | null>(null);

	const updateSdps = () => {
		setLocalSdp(voice.rtc!.conn.localDescription!.sdp);
		setRemoteSdp(voice.rtc!.conn.remoteDescription!.sdp);
	};
	updateSdps();

	let currentConn = voice.rtc?.conn;
	voice.rtc?.events.on("reconnect", ({ conn }) => {
		currentConn?.removeEventListener("connectionstatechange", updateSdps);
		conn.addEventListener("connectionstatechange", updateSdps);
		currentConn = conn;
	});

	currentConn?.addEventListener("connectionstatechange", updateSdps);
	onCleanup(() =>
		currentConn?.removeEventListener("connectionstatechange", updateSdps)
	);

	return (
		<div class="voice-debug">
			<header>voice/webrtc debugger</header>
			<nav>
				<For
					each={[
						{ tab: "sdp-local", label: "local sdp" },
						{ tab: "sdp-remote", label: "remote sdp" },
					]}
				>
					{(a) => (
						<button
							classList={{ active: tab() === a.tab }}
							onClick={() => setTab(a.tab)}
						>
							{a.label}
						</button>
					)}
				</For>
				<button onClick={props.onClose}>
					close
				</button>
			</nav>
			<main>
				<Switch>
					<Match when={tab() === "sdp-local"}>
						<Show when={localSdp()} fallback={"no local sdp?"}>
							{(s) => <VoiceSdp sdp={s()} />}
						</Show>
					</Match>
					<Match when={tab() === "sdp-remote"}>
						<Show when={remoteSdp()} fallback={"no remote sdp?"}>
							{(s) => <VoiceSdp sdp={s()} />}
						</Show>
					</Match>
					<Match when={tab() === "foobar"}>
						foobar!
					</Match>
				</Switch>
			</main>
		</div>
	);
};

export const VoiceSdp = (props: { sdp: string }) => {
	const sdp = createMemo(() => parseSessionDescription(props.sdp));

	return (
		<div class="voice-debug-sdp">
			<h3>sdp inspector</h3>
			<button
				style="margin-left: 4px"
				onClick={() => navigator.clipboard.writeText(props.sdp)}
			>
				copy
			</button>
			<Show when={sdp().errors.length}>
				<details class="errors" open>
					<summary>
						<h3>errors</h3>
					</summary>
					<ul>
						<For each={sdp().errors}>{(err) => <li>{err}</li>}</For>
					</ul>
				</details>
			</Show>
			<details class="session" open>
				<summary>
					<h3>session info</h3>
				</summary>
				<div>
					<b>version</b>: {sdp().session.version}
				</div>
				<div>
					<b>name</b>: {sdp().session.name ?? "unknown"}
				</div>
				<Show when={sdp().session.origin}>
					<details class="origin" open>
						<summary>
							<h3>origin</h3>
						</summary>
						<div>
							<b>username</b>: {sdp().session.origin?.username}
						</div>
						<div>
							<b>address</b>: {sdp().session.origin?.address}
						</div>
						<div>
							<b>session id</b>: {sdp().session.origin?.sessionId}
						</div>
						<div>
							<b>session version</b>: {sdp().session.origin?.sessionVersion}
						</div>
					</details>
				</Show>
				<Show when={sdp().session.connection}>
					<div>
						<b>connection</b>: {sdp().session.connection?.address}
					</div>
				</Show>
				<Show when={sdp().session.bandwidth}>
					<div>
						<b>bandwidth</b>: {sdp().session.bandwidth}
					</div>
				</Show>
			</details>
			<details class="session-attrs" open>
				<summary>
					<h3>session attributes</h3>
				</summary>
				<ul>
					<For each={sdp().attributes}>
						{({ key, value }) => (
							<li>
								<div>
									<b>{key}</b>: {getAttributeDescription(key, value)}
								</div>
								<div class="value">{value}</div>
							</li>
						)}
					</For>
				</ul>
			</details>
			<For each={sdp().media}>
				{(m) => (
					<details class="media" open>
						<summary>
							<h3>
								media (mid {m.attributes.find((i) => i.key === "mid")?.value})
							</h3>
						</summary>
						<div>
							<b>type</b>: {m.type}
						</div>
						<div>
							<b>port</b>: {m.port}
						</div>
						<div>
							<b>protocol</b>: {m.protocol}
						</div>
						<div>
							<b>formats</b>: {m.formats.join(", ") || "no formats!"}
						</div>
						<Show when={m.connection}>
							<div>
								<b>connection</b>: {m.connection?.address}
							</div>
						</Show>
						<div>
							<b>bandwidth</b>: {m.bandwidth}
						</div>
						<details class="media-attrs" open>
							<summary>
								<h3>attributes</h3>
							</summary>
							<ul>
								<For each={m.attributes}>
									{({ key, value }) => (
										<li>
											<div>
												<b>{key}</b>: {getAttributeDescription(key, value)}
											</div>
											<Show when={value}>
												<div class="value">
													{key === "candidate"
														? <HighlightIpAddr addr={value!} />
														: value}
												</div>
											</Show>
										</li>
									)}
								</For>
							</ul>
						</details>
					</details>
				)}
			</For>
		</div>
	);
};

const IP_REGEX =
	/(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)|(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}/g;

const HighlightIpAddr = (props: { addr: string }) => {
	const parts = createMemo(() => props.addr.split(IP_REGEX));
	const matches = createMemo(() => props.addr.match(IP_REGEX) ?? []);

	return (
		<span>
			<For each={parts()}>
				{(part, idx) => (
					<>
						{part}
						<Show when={!!matches()[idx()]}>
							<span class="ip-addr">{matches()[idx()]}</span>
						</Show>
					</>
				)}
			</For>
		</span>
	);
};
