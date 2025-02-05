import { ParentProps, VoidProps } from "solid-js";
import { Media } from "sdk";
import { useCtx } from "./context.ts";

type MediaProps = VoidProps<{ media: Media }>;

export const ImageView = (props: MediaProps) => {
	// const b = props.media.mime.split("/")[0];
	// const [ty] = props.media.mime.split(";");
	// const byteFmt = Intl.NumberFormat("en", {
	// 	notation: "compact",
	// 	style: "unit",
	// 	unit: "byte",
	// 	unitDisplay: "narrow",
	// });

	return (
		<Wrap media={props.media} popup>
			<img
				src={props.media.url}
				alt={props.media.alt ?? undefined}
				height={props.media.height!}
				width={props.media.width!}
			/>
		</Wrap>
	);
	// <a download={a.filename} href={a.url}>download {a.filename}</a>
	// <div class="dim">{ty} - {byteFmt.format(a.size)}</div>
};

export const VideoView = (props: MediaProps) => {
	return (
		<Wrap media={props.media}>
			<video controls src={props.media.url} />
		</Wrap>
	);
};

export const AudioView = (props: MediaProps) => {
	return <audio src={props.media.url} controls />;
};

export const Wrap = (props: ParentProps<{ media: Media; popup?: boolean }>) => {
	const ctx = useCtx();
	return (
		<div
			class="media"
			style={{
				"--height": `${props.media.height}px`,
				"--width": `${props.media.width}px`,
				"--aspect-ratio": `${props.media.width}/${props.media.height}`,
			}}
			onClick={() => {
				if (props.popup) {
					ctx.dispatch({
						do: "modal.open",
						modal: { type: "media", media: props.media },
					});
				}
			}}
		>
			<div class="inner">
				<div class="loader">loading</div>
				{props.children}
			</div>
		</div>
	);
};

// export const FileView = () => {};
