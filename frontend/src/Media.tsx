import { VoidProps } from "solid-js";
import { Media } from "sdk";

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
		<div
			class="media"
			style={{ "aspect-ratio": `${props.media.width} / ${props.media.height}` }}
		>
			<div
				class="spacer"
				style={{
					height: `${props.media.height}px`,
					width: `${props.media.width}px`,
				}}
			>
				loading
			</div>
			<img src={props.media.url} alt={props.media.alt ?? undefined} />
		</div>
	);
	// <a download={a.filename} href={a.url}>download {a.filename}</a>
	// <div class="dim">{ty} - {byteFmt.format(a.size)}</div>
};

export const VideoView = (props: MediaProps) => {
	return (
		<div
			class="media"
			style={{ "aspect-ratio": `${props.media.width} / ${props.media.height}` }}
		>
			<div
				class="spacer"
				style={{
					height: `${props.media.height}px`,
					width: `${props.media.width}px`,
				}}
			>
			</div>
			<video
				height={props.media.height!}
				width={props.media.width!}
				src={props.media.url}
				controls
			/>
		</div>
	);
};

export const AudioView = (props: MediaProps) => {
	return <audio src={props.media.url} controls />
};

// export const FileView = () => {};
