import { byteFmt, MediaProps } from "./util.ts";

export const FileView = (props: MediaProps) => {
	const ty = () => props.media.source.mime.split(";")[0];

	return (
		<div>
			<a download={props.media.filename} href={props.media.source.url}>
				download {props.media.filename}
			</a>
			<div class="dim">{ty()} - {byteFmt.format(props.media.source.size)}</div>
		</div>
	);
};
