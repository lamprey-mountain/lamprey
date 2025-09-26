import {
	createMemo,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
	VoidProps,
} from "solid-js";
import { getAttributeDescription, parseSessionDescription } from "./rtc-util";
import { useVoice } from "./voice-provider";
import { ReactiveMap } from "@solid-primitives/map";
import { Copyable } from "./util";
import { useApi } from "./api";

export const VoiceDebug = (props: { onClose: () => void }) => {
	const [voice] = useVoice();
	const api = useApi();

	const [tab, setTab] = createSignal("streams");
	const [localSdp, setLocalSdp] = createSignal<RTCSessionDescription | null>(
		null,
	);
	const [remoteSdp, setRemoteSdp] = createSignal<RTCSessionDescription | null>(
		null,
	);

	const updateSdps = () => {
		setLocalSdp(voice.rtc!.conn.localDescription);
		setRemoteSdp(voice.rtc!.conn.remoteDescription);
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

	const voiceStates = createMemo(() => {
		return [...api.voiceStates.values()].filter(i => i.thread_id === voice.threadId);
	});

	return (
		<div class="voice-debug">
			<header>voice/webrtc debugger</header>
			<nav>
				<For
					each={[
						{ tab: "states", label: "voice states" },
						{ tab: "streams", label: "streams" },
						{ tab: "stats", label: "stats" },
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
					<Match when={tab() === "states"}>
						<div style="margin: 8px;">
							<h3>{voiceStates().length} voice states(s)</h3>
							<br />
							<For each={voiceStates()}>
								{(s) => {
									return (
										<div style="border: solid #444 1px;padding:4px">
											<div>
												<b>user_id</b>: <Copyable>{s.user_id}</Copyable>
											</div>
											<pre>{JSON.stringify(s, null, 2)}</pre>
										</div>
									);
								}}
							</For>
						</div>
					</Match>
					<Match when={tab() === "streams"}>
						<div style="margin: 8px;">
							<h3>{voice.rtc?.streams.size} stream(s)</h3>
							<br />
							<For each={[...voice.rtc?.streams.values() ?? []]}>
								{(s) => {
									return (
										<div style="border: solid #444 1px;padding:4px">
											<div>
												<b>user_id</b>: <Copyable>{s.user_id}</Copyable>
											</div>
											<div>
												<b>key</b>: {s.key}
											</div>
											<div>
												<b>transceivers:</b>
												<ul style="list-style: inside">
													<For each={s.mids}>
														{(m) => {
															const t = voice.rtc?.transceivers.get(m);
															return (
																<li>
																	<b>{m}</b> {t?.sender.track?.kind ??
																		t?.receiver.track.kind}
																</li>
															);
														}}
													</For>
												</ul>
											</div>
										</div>
									);
								}}
							</For>
							<br />
							<h3>
								{voice.rtc?.conn.getTransceivers().length} transceivers
							</h3>
							<ul style="list-style: inside">
								<For each={voice.rtc?.conn.getTransceivers()}>
									{(t) => (
										<li>
											{t.mid} {t.direction} {t?.sender.track?.kind ??
												t?.receiver.track.kind}
										</li>
									)}
								</For>
							</ul>
						</div>
					</Match>
					<Match when={tab() === "stats"}>
						<VoiceStats />
					</Match>
					<Match when={tab() === "sdp-local"}>
						<Show when={localSdp()} fallback={"no local sdp?"}>
							{(s) => (
								<>
									<div style="margin: 8px;">
										<h3>local sdp ({s().type})</h3>
										<button
											style="margin-left: 8px"
											onClick={() => navigator.clipboard.writeText(s().sdp)}
										>
											copy
										</button>
									</div>
									<VoiceSdp sdp={s().sdp} />
								</>
							)}
						</Show>
					</Match>
					<Match when={tab() === "sdp-remote"}>
						<Show when={remoteSdp()} fallback={"no remote sdp?"}>
							{(s) => (
								<>
									<div style="margin: 8px;">
										<h3>remote sdp ({s().type})</h3>
										<button
											style="margin-left: 8px"
											onClick={() => navigator.clipboard.writeText(s().sdp)}
										>
											copy
										</button>
									</div>
									<VoiceSdp sdp={s().sdp} />
								</>
							)}
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

const VoiceStats = () => {
	// critical stats:
	// - bitrate
	// - {bytes,packets} {sent,recv,retransmit}
	// - ping/jitter
	// - codec

	const [voice] = useVoice();
	const [codec, setCodec] = createSignal<
		Record<
			string,
			{
				type: string;
				codec: string;
				channels: number;
				clockRate: number;
				mime: string;
			}
		>
	>();

	const bandwidthIn = new ReactiveMap<
		string,
		Array<{ ts: number; bytes: number }>
	>();
	const bandwidthOut = new ReactiveMap<
		string,
		Array<{ ts: number; bytes: number }>
	>();
	const jitters = new ReactiveMap<
		string,
		Array<{ ts: number; jitter: number }>
	>();

	const statsInterval = setInterval(async () => {
		const stats = await voice.rtc?.conn.getStats();
		const candidates: Array<unknown> = [];
		stats?.forEach((v) => {
			v.timestamp;
			if (
				v.type === "candidate-pair" || v.type === "local-candidate" ||
				v.type === "remote-candidate"
			) {
				candidates.push(v);
				return;
			} else if (v.type === "outbound-rtp") {
				v.mid;
				v.bytesSent;
				v.headerBytesSent;
				v.packetsSent;
				v.retransmittedBytesSent;
				v.retransmittedPacketsSent;
				const b = bandwidthOut.get(v.mid) ?? [];
				b.push({ ts: v.timestamp, bytes: v.bytesSent });
				if (b.length > 31) b.shift();
				bandwidthOut.set(v.mid, [...b]);
			} else if (v.type === "inbound-rtp") {
				const b = bandwidthIn.get(v.mid) ?? [];
				b.push({ ts: v.timestamp, bytes: v.bytesReceived });
				if (b.length > 31) b.shift();
				bandwidthIn.set(v.mid, [...b]);

				const j = jitters.get(v.mid) ?? [];
				j.push({ ts: v.timestamp, jitter: v.jitter * 1000 });
				if (j.length > 31) j.shift();
				jitters.set(v.mid, [...j]);
			} else if (v.type === "remote-outbound-rtp") {
				v.packetsSent;
			} else if (v.type === "codec") {
				setCodec((c) => ({
					...c,
					[v.payloadType]: {
						type: v.codecType,
						codec: v.codec,
						channels: v.channels,
						clockRate: v.clockRate,
						mime: v.mimeType,
					},
				}));
			}
		});
		// console.log(candidates)
	}, 1000);
	onCleanup(() => clearInterval(statsInterval));

	const [format, setFormat] = createSignal("bytes");

	return (
		<div style="padding: 8px">
			<button
				style="display:none"
				onClick={() =>
					setFormat((f) =>
						({ bytes: "packet", packets: "bytes" } as Record<string, string>)[f]
					)}
			>
				format: {format()}
			</button>
			<br />
			<For each={[...bandwidthIn.entries()]}>
				{([mid, bw]) => {
					const jitter = jitters.get(mid) ?? [];
					return (
						<>
							<details open>
								<summary>
									<h3>bytes sent (mid {mid})</h3>
								</summary>
								<Chart
									points={bw.map((e) => e.bytes)}
									height={bw.reduce((acc, i) => Math.max(acc, i.bytes), 0)}
								/>
							</details>
							<details open>
								<summary>
									<h3>jitter (mid {mid})</h3>
								</summary>
								<Chart
									unit="ms"
									points={jitter.map((e) => e.jitter)}
									height={jitter.reduce((acc, i) => Math.max(acc, i.jitter), 0)}
								/>
							</details>
						</>
					);
				}}
			</For>
			<For each={[...bandwidthOut.entries()]}>
				{([mid, bw]) => {
					return (
						<details open>
							<summary>
								<h3>bytes sent (mid {mid})</h3>
							</summary>
							<Chart
								points={bw.map((e) => e.bytes)}
								height={bw.reduce((acc, i) => Math.max(acc, i.bytes), 0)}
							/>
						</details>
					);
				}}
			</For>
			<br />
			codecs
			<ul>
				<For each={Object.entries(codec() ?? {})}>
					{([pt, codec]) => (
						<li>
							{pt}: {JSON.stringify(codec)}
						</li>
					)}
				</For>
			</ul>
		</div>
	);
};

const Chart = (
	props: VoidProps<{ points: Array<number>; height: number; unit?: string }>,
) => {
	const scaleX = () => 20;
	const scaleY = () => 100 / props.height;

	const pathStroke = () =>
		[
			`M 0 ${-props.points[0] * scaleY()}`,
			...props.points.slice(1).map((d, i) =>
				`L ${(i + 1) * scaleX()} ${-d * scaleY()}`
			),
		].join(" ");
	const pathFill = () =>
		[
			`M 0 0`,
			`L 0 ${-props.points[0] * scaleY()}`,
			...props.points.slice(1).map((d, i) =>
				`L ${(i + 1) * scaleX()} ${-d * scaleY()}`
			),
			`L ${scaleX() * (props.points.length - 1)} 0`,
		].join(" ");

	return (
		<svg class="chart" viewBox="0 -100 300 116">
			<defs>
				<linearGradient id="chart-gradient" x1="0" x2="0" y1="0" y2="1">
					<stop offset="0%" stop-color="#08f6" />
					<stop offset="100%" stop-color="#08f1" />
				</linearGradient>
			</defs>
			<For each={[-25, -50, -75, -100]}>
				{(y) => (
					<>
						<line
							x1="0"
							x2="600"
							y1={y}
							y2={y}
							stroke="#aaaa"
							stoke-width="1"
						/>
						<text x="0" y={y + 8 + 4} fill="#aaa" font-size="10">
							{(props.height * (-y / 200)).toFixed(2)} {props.unit}
						</text>
					</>
				)}
			</For>
			<For each={[0, 50, 100, 150, 200, 250, 300]}>
				{(x) => (
					<>
						<line
							x1={x}
							x2={x}
							y1="-100"
							y2="0"
							stroke="#aaaa"
							stoke-width="1"
						/>
						<text x={x + 4} y={8 + 4} fill="#aaa" font-size="10">
							{((1 - (x / 300)) * 31).toFixed(0)}s
						</text>
					</>
				)}
			</For>
			<path d={pathStroke()} fill="none" stroke="#08f" stroke-width="2" />
			<path d={pathFill()} fill="url(#chart-gradient)" />
		</svg>
	);
};
