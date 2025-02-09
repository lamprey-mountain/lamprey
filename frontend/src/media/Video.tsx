import { VoidProps } from "solid-js";
import { Media } from "sdk";

type MediaProps = VoidProps<{ media: Media }>;

export const VideoView = (props: MediaProps) => {
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
				<video controls src={props.media.url} />
			</div>
		</div>
	);
};
