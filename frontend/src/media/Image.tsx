import { createSignal } from "solid-js";
import { useCtx } from "../context.ts";
import { getThumb, getUrl, MediaProps, Resize } from "./util.tsx";

export const ImageView = (props: MediaProps) => {
	const ctx = useCtx();

	const [loaded, setLoaded] = createSignal(false);
	const thumb = () => getThumb(props.media, 300, 300)!;
	const url = () => getUrl(thumb());
	const width = () => thumb().width;
	const height = () => thumb().height;

	return (
		<Resize height={height()} width={width()} ratio={width() / height()}>
			<div
				class="image"
				onClick={() => {
					ctx.dispatch({
						do: "modal.open",
						modal: { type: "media", media: props.media },
					});
				}}
			>
				<div class="media-loader" classList={{ loaded: loaded() }}>loading</div>
				<img
					src={url()}
					alt={props.media.alt ?? undefined}
					height={height()!}
					width={width()!}
					onLoad={[setLoaded, true]}
					onEmptied={[setLoaded, false]}
				/>
			</div>
		</Resize>
	);
};
