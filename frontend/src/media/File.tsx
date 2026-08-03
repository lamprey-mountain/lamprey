import { Icon } from "@/atoms/Icon.tsx";
import { icFileGeneric } from "@/utils/icons.ts";
import { formatBytes, getUrl, type MediaProps } from "./util.tsx";

export const FileView = (props: MediaProps) => {
	const ty = () => props.media.content_type.split(";")[0];

	return (
		<div class="media file">
			<div class="top">
				<Icon src={icFileGeneric} />
				<a download={props.media.filename} href={getUrl(props.media)}>
					download {props.media.filename}
				</a>
			</div>
			<div class="dim">
				{ty()} - {formatBytes(props.media.size)}
			</div>
		</div>
	);
};
