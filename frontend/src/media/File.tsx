import { MediaProps } from "./util.ts";

export const FileView = (props: MediaProps) => {
	const byteFmt = Intl.NumberFormat("en", {
		notation: "compact",
		style: "unit",
		unit: "byte",
		unitDisplay: "narrow",
	});

	const ty = () => props.media.mime.split(";")[0];

	return (
		<div>
			<a download={props.media.filename} href={props.media.url}>
				download {props.media.filename}
			</a>
			<div class="dim">{ty()} - {byteFmt.format(props.media.size)}</div>
		</div>
	);
};
