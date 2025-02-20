import { createSignal, Show } from "solid-js";
import { useCtx } from "../context.ts";
import { getHeight, getUrl, getWidth, MediaProps, Resize } from "./util.tsx";

// TODO: ensure only images can be passed here
// TODO: use thumbnail
export const ImageView = (props: MediaProps) => {
	const ctx = useCtx();

	const [loaded, setLoaded] = createSignal(false);
	const height = () => getHeight(props.media);
	const width = () => getWidth(props.media);

	return (
		<Resize height={height()} width={width()}>
			<div
				class="image"
				onClick={() => {
					ctx.dispatch({
						do: "modal.open",
						modal: { type: "media", media: props.media },
					});
				}}
			>
				<Show when={!loaded()}>
					<div class="media-loader">loading</div>
				</Show>
				<img
					src={getUrl(props.media.source)}
					alt={props.media.alt ?? undefined}
					height={height()!}
					width={width()!}
					onLoad={[setLoaded, true]}
					onEmptied={[setLoaded, false]}
				/>
			</div>
		</Resize>
	);

	return (
		<div
			class="media image"
			style={{
				"--height": `${height()}px`,
				"--width": `${width()}px`,
				"--aspect-ratio": `${width()}/${height()}`,
			}}
			onClick={() => {
				ctx.dispatch({
					do: "modal.open",
					modal: { type: "media", media: props.media },
				});
			}}
		>
			<div class="inner">
				<div class="loader">loading</div>
				<img
					src={getUrl(props.media.source)}
					alt={props.media.alt ?? undefined}
					height={height()!}
					width={width()!}
				/>
			</div>
		</div>
	);
};
