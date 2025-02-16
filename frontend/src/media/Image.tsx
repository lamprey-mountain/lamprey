import { useCtx } from "../context.ts";
import { getHeight, getWidth, MediaProps } from "./util.ts";

// TODO: ensure only images can be passed here
// TODO: use thumbnail
export const ImageView = (props: MediaProps) => {
	const ctx = useCtx();

	const height = () => getHeight(props.media);
	const width = () => getWidth(props.media);

	return (
		<div
			class="media image"
			style={{
				"--height": `${height()}px`,
				"--width": `${width()}px`,
				"--aspect-ratio": `${width()}/${height()}`,
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
					src={props.media.source.url}
					alt={props.media.alt ?? undefined}
					height={height()!}
					width={width()!}
				/>
			</div>
		</div>
	);
};
