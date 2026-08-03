import {
	createContext,
	createSelector,
	createSignal,
	For,
	Match,
	Switch,
	useContext,
} from "solid-js";
import type {
	InteractionAllow,
	InteractionCreate,
	InteractionCreateType,
	InteractionErrorCode,
	LampreyComponent,
	LampreyComponentMedia,
} from "ts-sdk";
import { useApi } from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { usePermissions } from "@/hooks/usePermissions";
import { AudioView } from "@/media/Audio";
import { FileView } from "@/media/File";
import { ImageView } from "@/media/Image";
import { TextView } from "@/media/Text";
import { VideoView } from "@/media/Video";
import type { MessageT } from "@/types";
import { Markdown } from "./Markdown";
import { Icon } from "./Icon";
import { icGear } from "@/utils/icons";

type ComponentContextT = {
	channelId?: string;
	loading: boolean;
	interaction: ComponentsInteraction;
	isInteracted(id: string | undefined): boolean;
	check(a: InteractionAllow | undefined): boolean;
	handleInteraction(
		interaction: Omit<
			InteractionCreate,
			"application_id" | "channel_id" | "message_id"
		>,
	): void;
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

export type ComponentsProps = {
	components: Array<LampreyComponent>;
	message?: MessageT;
};

type ComponentsInteractionReason = InteractionErrorCode | "Network";

type ComponentsInteraction =
	| { state: "empty" }
	| { state: "failed"; custom_id?: string; reason: ComponentsInteractionReason }
	| { state: "pending"; nonce: string; custom_id?: string };

export const Components = (props: ComponentsProps) => {
	const api = useApi();
	const [interaction, setInteraction] = createSignal<ComponentsInteraction>({
		state: "empty",
	});

	const isInteracted = createSelector(() => {
		const i = interaction();
		if (i.state === "pending") return i.custom_id;
		return null;
	});

	const me = useCurrentUser();
	const perms = usePermissions(
		() => me()?.id,
		() => props.message?.room_id ?? undefined,
		() => props.message?.channel_id,
	);

	api.events.on("sync", ([sync]) => {
		const i = interaction();
		if (i.state !== "pending") return;

		if (sync.type === "InteractionSuccess") {
			if (i.nonce !== sync.nonce) return;
			setInteraction({ state: "empty" });
		} else if (sync.type === "InteractionFailure") {
			if (i.nonce !== sync.nonce) return;
			setInteraction({
				state: "failed",
				custom_id: i.custom_id,
				reason: sync.error_code,
			});
		}
	});

	// getters are used for reactivity
	const context: ComponentContextT = {
		get channelId() {
			return props.message?.channel_id;
		},

		get loading() {
			return interaction().state === "pending";
		},

		isInteracted,

		get interaction() {
			return interaction();
		},

		check(a: InteractionAllow | undefined) {
			if (!a) return true;

			const u = me();
			if (!u) return false;

			if (a.user_ids?.includes(u.id)) return true;

			const p = perms.permissions();
			if (a.role_ids?.some((r) => p.roles.has(r))) return true;

			if (a.permissions?.some((a) => p.permissions.has(a))) return true;

			return false;
		},

		handleInteraction(it) {
			const m = props.message;
			if (!m) return;

			const body = {
				...it,
				channel_id: m.channel_id,
				application_id: m.author_id,
				message_id: m.id,
			};

			const nonce = "interaction-" + Math.random().toString(36).slice(2);
			setInteraction({ state: "pending", nonce, custom_id: body.custom_id });

			api.client.http
				.POST("/api/v1/interaction", {
					body,
					headers: {
						"Idempotency-Key": nonce,
					},
				})
				.catch(() =>
					setInteraction({
						state: "failed",
						custom_id: body.custom_id,
						reason: "Network",
					}),
				);
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

	const disabled = () =>
		c.loading || !c.check(props.component.allow ?? undefined);

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
						class="button component-button"
						classList={{
							primary: m().style === "Primary",
							danger: m().style === "Danger",
							interacted: c.isInteracted(m().custom_id),
						}}
						disabled={disabled()}
						onClick={() => {
							c.handleInteraction({
								type: "Button",
								custom_id: m().custom_id,
							});
						}}
					>
						<Icon src={icGear} />
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
