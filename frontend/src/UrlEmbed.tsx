import type { Embed } from "sdk";
import { Show, type VoidProps } from "solid-js";
import { ImageView } from "./media/mod.tsx";
import sanitizeHtml from "sanitize-html";
import { md } from "./markdown.tsx";

type EmbedProps = {
	embed: Embed;
};

const sanitizeHtmlOptions: sanitizeHtml.IOptions = {
	transformTags: {
		del: "s",
	},
};

export const EmbedView = (props: VoidProps<EmbedProps>) => {
	const description = () => {
		const d = props.embed.description;
		if (!d) return null;
		return sanitizeHtml(
			md.parse(d ?? "") as string,
			sanitizeHtmlOptions,
		).trim();
	};

	return (
		<article
			class="embed"
			classList={{ color: !!props.embed.color }}
			style={{ "--color": props.embed.color || undefined }}
		>
			<Show when={props.embed.title || props.embed.url}>
				<div class="info">
					<header>
						<Show when={props.embed.url} fallback={<b>{props.embed.title}</b>}>
							<a class="title" href={props.embed.url!}>
								{props.embed.title || props.embed.url}
							</a>
						</Show>
						<Show when={props.embed.site_name || props.embed.url}>
							<span class="site">
								{" - "}
								{props.embed.site_name || URL.parse(props.embed.url!)?.host}
							</span>
						</Show>
					</header>
					<Show when={props.embed.description}>
						<p class="description markdown" innerHTML={description() ?? ""}></p>
					</Show>
				</div>
			</Show>
			<Show when={props.embed.thumbnail}>
				<div class="thumb">
					<ImageView
						media={props.embed.thumbnail!}
						thumb_width={64}
						thumb_height={64}
					/>
				</div>
			</Show>
			<Show when={props.embed.media}>
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
