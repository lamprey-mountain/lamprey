import type { Embed } from "sdk";
import { Show, type VoidProps } from "solid-js";
import { ImageView } from "./media/mod.tsx";

type EmbedProps = {
	embed: Embed;
};

export const EmbedView = (props: VoidProps<EmbedProps>) => {
	return (
		<article
			class="embed"
			classList={{ color: !!props.embed.color }}
			style={{ "--color": props.embed.color || undefined }}
		>
			<Show when={props.embed.title}>
				<div class="info">
					<header>
						<Show when={props.embed.url} fallback={<b>{props.embed.title}</b>}>
							<a class="title" href={props.embed.url}>{props.embed.title}</a>
						</Show>
						<Show when={props.embed.site_name || props.embed.url}>
							<span class="site">
								{" - "}
								{props.embed.site_name || URL.parse(props.embed.url)?.host}
							</span>
						</Show>
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
