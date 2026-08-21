import type { Attachment, Media } from "ts-sdk";
import { MediaView } from "./Media";

export type MediaSidebarProps = { media: Media; attachment?: Attachment };

export function MediaSidebar(props: MediaSidebarProps) {
	return (
		<div class="media-sidebar">
			<MediaView
				media={props.media}
				attachment={props.attachment}
				expanded={true}
			/>
		</div>
	);
}
