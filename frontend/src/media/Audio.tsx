import { VoidProps } from "solid-js";
import { Media } from "sdk";

type MediaProps = VoidProps<{ media: Media }>;

export const AudioView = (props: MediaProps) => {
	return <audio src={props.media.url} controls />;
};
