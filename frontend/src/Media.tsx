import { ParentProps, VoidProps } from "solid-js";
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
		<Resized media={props.media}>
			<img
				src={props.media.url}
				alt={props.media.alt ?? undefined}
				height={props.media.height!}
				width={props.media.width!}
			/>
		</Resized>
	);
	// <a download={a.filename} href={a.url}>download {a.filename}</a>
	// <div class="dim">{ty} - {byteFmt.format(a.size)}</div>
};

export const VideoView = (props: MediaProps) => {
	return (
		<Resized media={props.media}>
			<video controls src={props.media.url} />
		</Resized>
	);
};

export const AudioView = (props: MediaProps) => {
	return <audio src={props.media.url} controls />;
};

export const Resized = (props: ParentProps<{ media: Media }>) => {
	return (
		<div
			class="media"
			style={{
				"--height": `${props.media.height}px`,
				"--width": `${props.media.width}px`,
				"--aspect-ratio": `${props.media.width}/${props.media.height}`,
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
