import type { Media } from "sdk";
import { createEffect, createSignal } from "solid-js";
import { useCtx } from "../context";
import { getHeight, getUrl, getWidth, Resize } from "../media/util";

// currently only supports images!
// though, it doesn't make much sense for video/audio/other media?
export const ModalMedia = (props: { media: Media }) => {
	const ctx = useCtx();

	const [loaded, setLoaded] = createSignal(false);
	const height = () => getHeight(props.media);
	const width = () => getWidth(props.media);

	createEffect(() => console.log("loaded", loaded()));
	return (
		<div class="modal modal-media">
			<div class="bg" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog" aria-modal>
					<Resize height={height()} width={width()}>
						<div class="image full">
							<div class="media-loader" classList={{ loaded: loaded() }}>
								loading
							</div>
							<img
								src={getUrl(props.media)}
								alt={props.media.alt ?? undefined}
								height={height()!}
								width={width()!}
								onLoad={[setLoaded, true]}
								onEmptied={[setLoaded, false]}
							/>
						</div>
					</Resize>
					<a href={props.media.source.url}>Go to url</a>
				</div>
			</div>
		</div>
	);
};
