import { VoidProps } from "solid-js";
import { Media } from "sdk";
import { useCtx } from "../context.ts";

type MediaProps = VoidProps<{ media: Media }>;

export const ImageView = (props: MediaProps) => {
	const ctx = useCtx();

	return (
		<div
			class="media image"
			style={{
				"--height": `${props.media.height}px`,
				"--width": `${props.media.width}px`,
				"--aspect-ratio": `${props.media.width}/${props.media.height}`,
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
					src={props.media.url}
					alt={props.media.alt ?? undefined}
					height={props.media.height!}
					width={props.media.width!}
				/>
			</div>
		</div>
	);
};
