import { createEffect, createMemo, createSignal, on, Show } from "solid-js";
import iconDownload from "@/assets/download.png";
import { Icon } from "@/atoms/Icon";
import { useModals } from "@/contexts/modal";
import { flags } from "@/lib/flags.ts";
import { icWarning } from "@/utils/icons.ts";
import {
	formatBytes,
	getThumb,
	getUrl,
	Loader,
	type MediaProps,
	NSFW_KEY,
	NSFW_THRESHOLD,
	Resize,
} from "./util.tsx";

type ImageViewProps = MediaProps & {
	thumb_width?: number;
	thumb_height?: number;
};

// TODO: blur spoiler images

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

	const [blurred, setBlurred] = createSignal(false);
	const blurred2 = () => blurred() && flags.has("nsfw_blur"); // TEMP

	const isNsfw = createMemo(() => {
		const scan = props.media.scans?.find((i) => i.key === NSFW_KEY);
		const score = scan?.result ?? 0;
		return score > NSFW_THRESHOLD;
	});

	createEffect(
		on(
			() => props.media.id,
			(id, prev) => {
				if (id !== prev) setBlurred(isNsfw());
			},
		),
	);

	return (
		<Resize height={height()} width={width()} ratio={width() / height()}>
			<article
				class="media image"
				classList={{ blurred: blurred2() }}
				onClick={(e) => {
					e.stopPropagation();
					if (blurred2()) {
						setBlurred(false);
					} else {
						modalctl.open({ type: "media", media: props.media });
					}
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
				<Show when={blurred2()}>
					<div class="nsfw-warning">
						<Icon src={icWarning} />
						nsfw
					</div>
				</Show>
				<a
					class="download"
					download={props.media.filename}
					href={getUrl(props.media)}
					onClick={(e) => e.stopPropagation()}
				>
					<button type="button" class="button">
						<Icon src={iconDownload} alt="download" />
					</button>
				</a>
				<footer class="info dim">
					{props.media.filename} - {formatBytes(props.media.size)}
				</footer>
			</article>
		</Resize>
	);
};
