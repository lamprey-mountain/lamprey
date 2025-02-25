import { UrlEmbed } from "sdk";
import { Show, VoidProps } from "solid-js";
import { ImageView } from "./media/mod";

type UrlEmbedProps = {
	embed: UrlEmbed;
};

export const UrlEmbedView = (props: VoidProps<UrlEmbedProps>) => {
	return (
		<article class="embed">
			<Show when={props.embed.title}>
				<div class="info">
					<header>
						<a class="title" href={props.embed.url}>{props.embed.title}</a>
						<span class="site">
							{" - "}
							{props.embed.site_name || new URL(props.embed.url).host}
						</span>
					</header>
					<p class="description">{props.embed.description}</p>
				</div>
			</Show>
			<Show when={props.embed.media && props.embed.media_is_thumbnail}>
				<div class="thumb">
					<ImageView
						media={props.embed.media!}
						thumb_width={64}
						thumb_height={64}
					/>
				</div>
			</Show>
			<Show when={props.embed.media && !props.embed.media_is_thumbnail}>
				<div class="media">
					<ImageView
						media={props.embed.media!}
						thumb_width={320}
						thumb_height={320}
					/>
				</div>
			</Show>
		</article>
	);
};
