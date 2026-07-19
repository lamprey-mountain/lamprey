import { ReactiveMap } from "@solid-primitives/map";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	on,
	onCleanup,
	Show,
	Switch,
	type VoidProps,
} from "solid-js";
import { useApi } from "@/api";
import { Copyable } from "@/utils/general";
import { useVoice } from "../voice/context";
import { getAttributeDescription, parseSessionDescription } from "./rtc-util";

export const VoiceDebug = (props: { onClose: () => void }) => {
	const [voice] = useVoice();
	const api = useApi();
	const vc = voice.vc;

	const [tab, setTab] = createSignal("streams");
	const [localSdp, setLocalSdp] = createSignal<RTCSessionDescription | null>(
		null,
	);
	const [remoteSdp, setRemoteSdp] = createSignal<RTCSessionDescription | null>(
		null,
	);
	const [transceivers, setTransceivers] = createSignal<RTCRtpTransceiver[]>(
		vc.getRtc()?.getTransceivers() ?? [],
	);
	const [tracks, setTracks] = createSignal([...vc.tracks.values()]);

	const updateDebugInfo = () => {
		setLocalSdp(vc.getRtc()?.localDescription ?? null);
		setRemoteSdp(vc.getRtc()?.remoteDescription ?? null);
		setTransceivers(vc.getRtc()?.getTransceivers() ?? []);
		setTracks([...vc.tracks.values()]);
	};
	updateDebugInfo();

	createEffect(
		on(
			() => (vc.connectionState(), vc.getRtc()),
			(rtc, old) => {
				if (!rtc) return;
				if (rtc === old) return;

				rtc.addEventListener("track", updateDebugInfo);
				onCleanup(() => {
					rtc.removeEventListener("track", updateDebugInfo);
				});
			},
		),
	);

	const voiceStates = createMemo(() => {
		return [...api.voiceStates.values()].filter(
			(i) => i.channel_id === voice.joinedChannelId,
		);
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
							type="button"
							class="button"
							classList={{ active: tab() === a.tab }}
							onClick={() => setTab(a.tab)}
						>
							{a.label}
						</button>
					)}
				</For>
				<button type="button" class="button" onClick={props.onClose}>
					close
				</button>
			</nav>
			<main>
				<Switch>
					<Match when={tab() === "states"}>
						<VoiceStatesTab voiceStates={voiceStates()} />
					</Match>
					<Match when={tab() === "streams"}>
						<VoiceStreamsTab
							vc={vc}
							transceivers={transceivers()}
							tracks={tracks()}
						/>
					</Match>
					<Match when={tab() === "stats"}>
						<VoiceStatsTab />
					</Match>
					<Match when={tab() === "sdp-local"}>
						<VoiceSdpTab sdp={localSdp()} title="local sdp" />
					</Match>
					<Match when={tab() === "sdp-remote"}>
						<VoiceSdpTab sdp={remoteSdp()} title="remote sdp" />
					</Match>
				</Switch>
			</main>
		</div>
	);
};

const VoiceStatesTab = (props: { voiceStates: any[] }) => (
	<div style="margin: 8px;">
		<h3 class="dim">{props.voiceStates.length} voice states(s)</h3>
		<For each={props.voiceStates}>
			{(s, idx) => (
				<details class="voice-state" open>
					<summary>
						<div>
							<h3 class="dim">
								Voice state <span class="light">#{idx()}</span>
							</h3>
						</div>
						<div class="dim">
							<em>user_id</em>: <Copyable>{s.user_id}</Copyable>
						</div>
					</summary>
					<JsonView json={s} />
				</details>
			)}
		</For>
	</div>
);

const VoiceStreamsTab = (props: {
	vc: any;
	transceivers: RTCRtpTransceiver[];
	tracks: any[];
}) => (
	// TODO: details/summary for each section (make them collapseable)

	<div class="voice-streams-debug">
		<section class="section">
			<h3>{props.vc.streams.size} stream(s)</h3>
			<ul>
				<For each={[...(props.vc.streams.values() ?? [])]}>
					{(s) => (
						<li class="item">
							<div>
								<b>user_id</b>: <Copyable>{s.user_id}</Copyable>
							</div>
							<div>
								<b>key</b>: {s.key.toString()}
							</div>
							<div>
								<b>track_ids:</b> {s.track_ids.join(", ")}
							</div>
						</li>
					)}
				</For>
			</ul>
		</section>
		<section class="section">
			<h3>{props.transceivers.length} transceivers</h3>
			<ul>
				<For each={props.transceivers}>
					{(t) => (
						<li class="item">
							<div>
								<b>mid</b>: {t.mid}
							</div>
							<div>
								<b>direction</b>: {t.direction}
							</div>
							<div>
								<b>kind</b>:{" "}
								{t?.sender.track?.kind ?? t?.receiver.track?.kind ?? "unknown"}
							</div>
							<div>
								<b>muted</b>:{" "}
								{String(
									t.sender.track?.muted ?? t.receiver.track?.muted ?? "unknown",
								)}
							</div>
							<div>
								<b>enabled</b>:{" "}
								{String(
									t.sender.track?.enabled ??
										t.receiver.track?.enabled ??
										"unknown",
								)}
							</div>
						</li>
					)}
				</For>
			</ul>
		</section>
		<section class="section">
			<h3>{props.tracks.length} tracks</h3>
			<ul>
				<For each={props.tracks}>
					{(t) => (
						<li class="item">
							<div>
								<b>id:</b> {String(t.id ?? "none (local)")}
							</div>
							<div>
								<b>mid:</b> {String(t.mid ?? "none")}
							</div>
							<div>
								<b>user_id:</b> <Copyable>{t.user_id}</Copyable>
							</div>
							<div>
								<b>key:</b> {String(t.metadata.key)}
							</div>
							<div>
								<b>kind:</b> {t.metadata.kind}
							</div>
						</li>
					)}
				</For>
			</ul>
		</section>
	</div>
);

const VoiceSdpTab = (props: {
	sdp: RTCSessionDescription | null;
	title: string;
}) => (
	<Show when={props.sdp} fallback={"no " + props.title + "?"}>
		{(s) => (
			<>
				<div style="margin: 8px;">
					<h3>
						{props.title} ({s().type})
					</h3>
					<button
						type="button"
						class="button"
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
);

const JsonView = (props: { json: any }) => {
	let ref!: HTMLPreElement;

	createEffect(() => {
		const jsonString = JSON.stringify(props.json, null, 2);
		ref.textContent = jsonString;

		import("highlight.js").then(({ default: hljs }) => {
			hljs.highlightElement(ref);
		});
	});

	return <pre ref={ref} class="language-json" />;
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
													{key === "candidate" ? (
														<HighlightIpAddr addr={value!} />
													) : (
														value
													)}
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

const VoiceStatsTab = () => {
	// critical stats:
	// - bitrate
	// - {bytes,packets} {sent,recv,retransmit}
	// - ping/jitter
	// - codec

	const [voice] = useVoice();
	const vc = voice.vc;

	const [codec, setCodec] =
		createSignal<
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

	const MAX_POINTS = 31;

	const statsInterval = setInterval(async () => {
		const rtc = vc.getRtc();
		if (!rtc) return;
		const stats = await rtc.getStats();
		const candidates: Array<unknown> = [];
		stats.forEach((v) => {
			if (
				v.type === "candidate-pair" ||
				v.type === "local-candidate" ||
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
				const rtpV = v as RTCOutboundRtpStreamStats;
				const b = bandwidthOut.get(rtpV.mid ?? "") ?? [];
				b.push({ ts: v.timestamp, bytes: rtpV.bytesSent ?? 0 });
				if (b.length > MAX_POINTS) b.shift();
				bandwidthOut.set(rtpV.mid ?? "", [...b]);
			} else if (v.type === "inbound-rtp") {
				const rtpV = v as RTCInboundRtpStreamStats;
				const b = bandwidthIn.get(rtpV.mid ?? "") ?? [];
				b.push({ ts: v.timestamp, bytes: rtpV.bytesReceived ?? 0 });
				if (b.length > MAX_POINTS) b.shift();
				bandwidthIn.set(rtpV.mid ?? "", [...b]);

				const j = jitters.get(rtpV.mid ?? "") ?? [];
				j.push({ ts: v.timestamp, jitter: (rtpV.jitter ?? 0) * 1000 });
				if (j.length > MAX_POINTS) j.shift();
				jitters.set(rtpV.mid ?? "", [...j]);
			} else if (v.type === "codec") {
				const codec = v as any; // FIXME: use correct type
				setCodec((c) => ({
					...c,
					[codec.id]: {
						type: codec.codecType ?? "unknown",
						codec: codec.mimeType ?? "unknown",
						channels: codec.channels ?? 0,
						clockRate: codec.clockRate ?? 0,
						mime: codec.mimeType ?? "unknown",
					},
				}));
			}
		});
	}, 1000);
	onCleanup(() => clearInterval(statsInterval));

	const [format, setFormat] = createSignal("bytes");

	return (
		<div style="padding: 8px">
			<button
				type="button"
				class="button"
				style="display:none"
				onClick={() =>
					setFormat(
						(f) =>
							(
								({ bytes: "packet", packets: "bytes" }) as Record<
									string,
									string
								>
							)[f],
					)
				}
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

// TODO: split into separate component
const Chart = (
	props: VoidProps<{ points: Array<number>; height: number; unit?: string }>,
) => {
	const scaleX = () => 20;
	const scaleY = () => 100 / props.height;

	const pathStroke = () =>
		[
			`M 0 ${-props.points[0] * scaleY()}`,
			...props.points
				.slice(1)
				.map((d, i) => `L ${(i + 1) * scaleX()} ${-d * scaleY()}`),
		].join(" ");
	const pathFill = () =>
		[
			`M 0 0`,
			`L 0 ${-props.points[0] * scaleY()}`,
			...props.points
				.slice(1)
				.map((d, i) => `L ${(i + 1) * scaleX()} ${-d * scaleY()}`),
			`L ${scaleX() * (props.points.length - 1)} 0`,
		].join(" ");

	return (
		<svg aria-hidden="true" class="chart" viewBox="0 -100 300 116">
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
							{((1 - x / 300) * 31).toFixed(0)}s
						</text>
					</>
				)}
			</For>
			<path d={pathStroke()} fill="none" stroke="#08f" stroke-width="2" />
			<path d={pathFill()} fill="url(#chart-gradient)" />
		</svg>
	);
};
