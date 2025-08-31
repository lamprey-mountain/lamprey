import { formatBytes, getUrl, type MediaProps } from "./util.tsx";

export const FileView = (props: MediaProps) => {
	const ty = () => props.media.source.mime.split(";")[0];

	return (
		<div>
			<a download={props.media.filename} href={getUrl(props.media)}>
				download {props.media.filename}
			</a>
			<div class="dim">{ty()} - {formatBytes(props.media.source.size)}</div>
		</div>
	);
};
