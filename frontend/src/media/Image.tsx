import { createEffect, createSignal } from "solid-js";
import iconDownload from "../assets/download.png";
import { useCtx } from "../context.ts";
import { useModals } from "../contexts/modal";
import {
	formatBytes,
	getThumb,
	getUrl,
	Loader,
	type MediaProps,
	Resize,
} from "./util.tsx";

type ImageViewProps = MediaProps & {
	thumb_width?: number;
	thumb_height?: number;
};

export const ImageView = (props: ImageViewProps) => {
	const [, modalctl] = useModals();
	const [loaded, setLoaded] = createSignal(false);
	const thumbUrl = () => getThumb(props.media, props.thumb_width ?? 320)!;

	const height = () => {
		const metadata = props.media.metadata as any;
		if (metadata.type === "Image") {
			return metadata.height;
		}
		return 0;
	};

	const width = () => {
		const metadata = props.media.metadata as any;
		if (metadata.type === "Image") {
			return metadata.width;
		}
		return 0;
	};

	return (
		<Resize height={height()} width={width()} ratio={width() / height()}>
			<article
				class="image"
				onClick={(e) => {
					e.stopPropagation();
					modalctl.open({ type: "media", media: props.media });
				}}
			>
				<Loader loaded={loaded()} />
				<img
					src={thumbUrl()}
					alt={props.media.alt ?? undefined}
					height={height()!}
					width={width()!}
					ref={(el) => {
						if (el.complete && el.naturalWidth > 0) setLoaded(true);
					}}
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
					{props.media.filename} - {formatBytes(props.media.size)}
				</footer>
			</article>
		</Resize>
	);
};
