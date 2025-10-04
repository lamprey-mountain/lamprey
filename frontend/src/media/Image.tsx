import { createSignal, Show } from "solid-js";
import { useCtx } from "../context.ts";
import {
	formatBytes,
	getThumb,
	getUrl,
	Loader,
	type MediaProps,
	Resize,
} from "./util.tsx";
import iconDownload from "../assets/download.png";
import { createEffect } from "solid-js";

type ImageViewProps = MediaProps & {
	thumb_width?: number;
	thumb_height?: number;
};

export const ImageView = (props: ImageViewProps) => {
	const ctx = useCtx();
	const [loaded, setLoaded] = createSignal(false);
	const thumbUrl = () => getThumb(props.media, props.thumb_width ?? 320)!;

	const height = () => {
		if (props.media.source.type === "Image") {
			return props.media.source.height;
		}
		return 0;
	};

	const width = () => {
		if (props.media.source.type === "Image") {
			return props.media.source.width;
		}
		return 0;
	};

	createEffect(() => {
		console.log("img", loaded());
	});

	return (
		<Resize height={height()} width={width()} ratio={width() / height()}>
			<article
				class="image"
				onClick={(e) => {
					e.stopPropagation();
					ctx.dispatch({
						do: "modal.open",
						modal: { type: "media", media: props.media },
					});
				}}
			>
				<Loader loaded={loaded()} />
				<img
					src={thumbUrl()}
					alt={props.media.alt ?? undefined}
					height={height()!}
					width={width()!}
					onLoad={() => setLoaded(true)}
					onEmptied={() => setLoaded(false)}
				/>
				<a
					class="download"
					download={props.media.filename}
					href={getUrl(props.media)}
					onClick={(e) => e.stopPropagation()}
				>
					<button>
						<img src={iconDownload} class="icon" />
					</button>
				</a>
				<footer class="info dim">
					{props.media.filename} - {formatBytes(props.media.source.size)}
				</footer>
			</article>
		</Resize>
	);
};
