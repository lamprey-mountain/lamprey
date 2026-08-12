import sanitizeHtml from "sanitize-html";
import type { Embed } from "sdk";
import { Match, Show, Switch, type VoidProps } from "solid-js";
import { parserResource } from "@/lib/markdown";
import { MediaView } from "@/media/Media";
import { ImageView } from "@/media/mod";

type EmbedProps = {
	embed: Embed;
};

const sanitizeHtmlOptions: sanitizeHtml.IOptions = {
	transformTags: {
		del: "s",
	},
};

export const EmbedView = (props: VoidProps<EmbedProps>) => {
	return (
		<Switch>
			<Match when={props.embed.type === "Media"}>
				<MediaView media={props.embed.media!} />
			</Match>
			<Match when={true}>
				<EmbedViewUrl embed={props.embed} />
			</Match>
		</Switch>
	);
};

export const EmbedViewUrl = (props: VoidProps<EmbedProps>) => {
	// TODO: attempt to autodetect if this is html or markdown
	const description = () => {
		const md = parserResource();
		if (!md) return null;

		const d = props.embed.description;
		if (!d) return null;

		return sanitizeHtml(md.parse(d).toHTML(), sanitizeHtmlOptions).trim();
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
							<a class="title" href={props.embed.url ?? ""}>
								{props.embed.title || props.embed.url}
							</a>
						</Show>
						<Show when={props.embed.site_name || props.embed.url}>
							<span class="site">
								{" - "}
								{props.embed.site_name ||
									URL.parse(props.embed.url ?? "")?.host}
							</span>
						</Show>
					</header>
					<Show when={description()}>
						{(desc) => (
							<div class="description markdown" innerHTML={desc()}></div>
						)}
					</Show>
				</div>
			</Show>
			<Show when={props.embed.thumbnail}>
				{(thumbnail) => (
					<div class="thumb">
						<ImageView media={thumbnail()} thumb_width={64} thumb_height={64} />
					</div>
				)}
			</Show>
			<Show when={props.embed.media}>
				{(media) => (
					<div class="media">
						<ImageView media={media()} thumb_width={320} thumb_height={320} />
					</div>
				)}
			</Show>
		</article>
	);
};
