import { createContext, For, Match, Switch, useContext } from "solid-js";
import type { LampreyComponent, LampreyComponentMedia } from "ts-sdk";
import { AudioView } from "@/media/Audio";
import { FileView } from "@/media/File";
import { ImageView } from "@/media/Image";
import { TextView } from "@/media/Text";
import { VideoView } from "@/media/Video";
import { Markdown } from "./Markdown";

type Interaction = { type: "click"; component: LampreyComponent };

type ComponentContextT = {
	channelId?: string;
	handleInteraction(interaction: Interaction): void;
};

const ComponentContext = createContext<ComponentContextT>();

const useComponents = () => useContext(ComponentContext)!;

// Helper for type-safe matching with SolidJS Switch/Match
function matches<S extends LampreyComponent>(
	e: LampreyComponent,
	predicate: (e: LampreyComponent) => e is S,
): S | false {
	return predicate(e) ? e : false;
}

export const Components = (props: {
	components: Array<LampreyComponent>;
	channelId?: string;
}) => {
	const context: ComponentContextT = {
		// use getter for reactivity
		get channelId() {
			return props.channelId;
		},
		handleInteraction(interaction) {
			// TODO: send to server or something
			console.log("handle interaction", {
				type: interaction.type,
				// room_id, channel_id, message_id, application_id
				// component_id, custom_id
				// data, nonce
			});
		},
	};

	return (
		<ComponentContext.Provider value={context}>
			<div class="components">
				<For each={props.components}>
					{(c) => <ComponentRenderer component={c} />}
				</For>
			</div>
		</ComponentContext.Provider>
	);
};

const ComponentRenderer = (props: { component: LampreyComponent }) => {
	const c = useComponents();
	return (
		<Switch>
			<Match when={matches(props.component, (e) => e.type === "Text")}>
				{(m) => (
					<div class="text">
						<Markdown content={m().content} channel_id={c.channelId} />
					</div>
				)}
			</Match>

			<Match when={matches(props.component, (e) => e.type === "Container")}>
				{(m) => (
					<div class="container" style={{ "--color": m().color ?? undefined }}>
						<For each={m().components}>
							{(child) => <ComponentRenderer component={child} />}
						</For>
					</div>
				)}
			</Match>

			<Match when={matches(props.component, (e) => e.type === "Section")}>
				{(m) => (
					<div class="section" style={{ "--color": m().color ?? undefined }}>
						<For each={m().components}>
							{(child) => <ComponentRenderer component={child} />}
						</For>
					</div>
				)}
			</Match>

			<Match when={matches(props.component, (e) => e.type === "Button")}>
				{(m) => (
					<button
						class={`button component-button button-${m().style.toLowerCase()}`}
						onClick={() =>
							c.handleInteraction({ type: "click", component: m() })
						}
					>
						{m().label}
					</button>
				)}
			</Match>

			<Match when={matches(props.component, (e) => e.type === "LinkButton")}>
				{(m) => (
					<a
						class="button component-button button-secondary"
						href={m().url ?? undefined}
						target="_blank"
						rel="noopener noreferrer"
					>
						{m().label}
					</a>
				)}
			</Match>

			<Match when={matches(props.component, (e) => e.type === "Details")}>
				{(m) => (
					<details class="details">
						<summary>
							<For each={m().summary}>
								{(child) => <ComponentRenderer component={child} />}
							</For>
						</summary>
						<For each={m().details}>
							{(child) => <ComponentRenderer component={child} />}
						</For>
					</details>
				)}
			</Match>

			<Match when={matches(props.component, (e) => e.type === "Media")}>
				{(m) => (
					<div class="media">
						<For each={m().items}>{(item) => <MediaItem media={item} />}</For>
					</div>
				)}
			</Match>

			<Match when={matches(props.component, (e) => e.type === "Gallery")}>
				{(_m) => <div>todo</div>}
			</Match>
		</Switch>
	);
};

const MediaItem = (props: { media: LampreyComponentMedia }) => {
	const b = () => props.media.media.content_type.split("/")[0];
	const isJson = () =>
		/^application\/json\b/.test(props.media.media.content_type);

	return (
		<div class="media">
			<Switch>
				<Match when={b() === "image"}>
					<ImageView media={props.media.media} />
				</Match>
				<Match when={b() === "video"}>
					<VideoView media={props.media.media} />
				</Match>
				<Match when={b() === "audio"}>
					<AudioView media={props.media.media} />
				</Match>
				<Match when={b() === "text" || isJson()}>
					<TextView media={props.media.media} />
				</Match>
				<Match when={true}>
					<FileView media={props.media.media} />
				</Match>
			</Switch>
			<div class="description">{props.media.description}</div>
		</div>
	);
};
