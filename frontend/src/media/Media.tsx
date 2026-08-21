import { createMemo, Match, Switch } from "solid-js";
import type { Attachment, Media } from "ts-sdk";
import { flags } from "@/lib/flags";
import { AudioView } from "./Audio";
import { FileView } from "./File";
import { ImageView } from "./Image";
import { TextView } from "./Text";
import { ThreeView } from "./Three";
import { is3D } from "./three-util";
import { VideoView } from "./Video";

export type MediaViewProps = {
	media: Media;
	attachment?: Attachment;
	expanded?: boolean;
};

export function MediaView(props: MediaViewProps) {
	const media = () => props.media;
	const contentType = createMemo(() => props.media?.content_type);
	const mainCt = createMemo(() => contentType()?.split("/")[0]);

	const isText = () => {
		return mainCt() === "text" || /^application\/json\b/.test(contentType());
	};

	// TODO: better mime type parsing/matching for application/json
	// TODO: support spoilers for other media types

	return (
		<Switch>
			<Match when={mainCt() === "image"}>
				<ImageView
					media={media()}
					spoiler={props.attachment?.spoiler ?? false}
				/>
			</Match>
			<Match when={mainCt() === "video"}>
				<VideoView media={media()} />
			</Match>
			<Match when={mainCt() === "audio"}>
				<AudioView media={media()} />
			</Match>
			<Match when={isText()}>
				<TextView media={media()} expanded={props.expanded} />
			</Match>
			<Match when={flags.has("media_three") && is3D(media())}>
				<ThreeView media={media()} />
			</Match>
			<Match when={true}>
				<FileView media={media()} />
			</Match>
		</Switch>
	);
}
