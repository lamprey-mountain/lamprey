import { createEffect, createSignal, For, Match, Switch } from "solid-js";
import { useVoice } from "./context";

export const VoiceStreams = () => {
	const [voice] = useVoice();

	return (
		<div class="voice-streams">
			<For each={[...voice.vc.streams.values()]}>
				{(stream) => {
					const [ref, setRef] = createSignal(
						null as HTMLAudioElement | HTMLVideoElement | null,
					);

					const hasVideo = () => stream.media.getVideoTracks().length > 0;

					const muted = () =>
						voice.deafened ||
						voice.preferences.get(stream.user_id)?.mute === true;
					const volume = () =>
						voice.preferences.get(stream.user_id)?.volume ?? 1;

					createEffect(() => {
						const r = ref();
						if (r) {
							r.srcObject = stream.media;
							r.volume = volume();
						}
					});

					return (
						<Switch>
							<Match when={hasVideo()}>
								<video autoplay playsinline ref={setRef} muted={muted()} />
							</Match>
							<Match when={true}>
								<audio autoplay ref={setRef} muted={muted()} />
							</Match>
						</Switch>
					);
				}}
			</For>
		</div>
	);
};
