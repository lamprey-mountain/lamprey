import type { RouteSectionProps } from "@solidjs/router";
import {
	createMemo,
	createResource,
	For,
	type JSX,
	Match,
	type ParentProps,
	Show,
	Switch,
} from "solid-js";
import type { ApiError, MediaLinkType } from "ts-sdk";
import { useApi } from "@/api";
import { ApiErrorDisplay, isApiError } from "@/atoms/ApiErrorDisplay";
import { Icon } from "@/atoms/Icon";
import { Resizable } from "@/atoms/Resizable";
import { MediaView } from "@/media/Media";
import { formatBytes, getMediaIcon } from "@/media/util";
import type { MediaT } from "@/types";
import { Copyable2 } from "@/utils/general";
import { icFileGeneric } from "@/utils/icons";
import { Title } from "./Title";
import { Avatar, UserDisplayName } from "./User";

type MediaResult =
	| { status: "loading" }
	| { status: "loaded"; media: MediaT }
	| { status: "apiError"; err: ApiError }
	| { status: "platformError"; err: unknown };

export const RouteMedia = (p: ParentProps<RouteSectionProps>): JSX.Element => {
	const api = useApi();

	const [mediaResource] = createResource<MediaResult, string>(
		() => p.params.media_id,
		async (id) => {
			try {
				const media = await api.media.fetch(id);
				return { status: "loaded", media };
			} catch (err) {
				if (isApiError(err)) {
					return { status: "apiError", err };
				}
				return { status: "platformError", err };
			}
		},
		{ initialValue: { status: "loading" } },
	);

	function matches<T extends MediaResult["status"]>(
		ty: T,
	): (MediaResult & { status: T }) | false {
		const m = mediaResource() ?? { status: "loading" };
		if (m.status === ty) {
			return m as MediaResult & { status: T };
		} else {
			return false;
		}
	}

	const mediaIcon = () => {
		const m = mediaResource();
		if (m.status === "loaded") return getMediaIcon(m.media);
		return icFileGeneric;
	};

	const mediaName = () => {
		const m = mediaResource();
		if (m.status === "loaded") return m.media.filename;
		return "unknown media";
	};

	return (
		<>
			<header class="chat-header">
				<div class="channel-icon">
					<Icon src={mediaIcon()} />
				</div>
				<div class="name">
					<h3 class="name-text">{mediaName()}</h3>
				</div>
				<div class="spacer"></div>
			</header>
			<div class="media-page-wrapper">
				<Switch>
					<Match when={matches("loading")}>
						{/* TODO: skeleton ui for user page */}
						<div>loading...</div>
					</Match>
					<Match when={matches("apiError")}>
						{(err) => (
							/* TODO: no inline styles */
							<div style="display:grid;place-items:center;height:100%">
								<ApiErrorDisplay error={err().err} />
							</div>
						)}
					</Match>
					<Match when={matches("platformError")}>
						{/* TODO: better ui for this */}
						<div>internal error</div>
					</Match>
					<Match when={matches("loaded")}>
						{(m) => (
							<>
								<Title title={mediaName()} />
								<RouteMediaInner media={m().media} />
							</>
						)}
					</Match>
				</Switch>
			</div>
			<Resizable
				storageKey="media-sidebar"
				side="right"
				initialWidth={256}
				// TODO: don't have magic numbers
				minWidth={244}
				maxWidth={564}
			>
				<Show when={matches("loaded")}>
					{(m) => <MediaDetails media={m().media} />}
				</Show>
			</Resizable>
		</>
	);
};

export const RouteMediaInner = (props: { media: MediaT }): JSX.Element => {
	// TODO: don't resize media like in chat, always try to display it full size

	const ty = createMemo(() => props.media.content_type.split(";")[0]);

	return (
		<>
			<div class="media-page">
				<MediaView media={props.media} />
				<div>{props.media.filename}</div>
				<div class="dim">
					{ty()} - {formatBytes(props.media.size)}
				</div>
			</div>
		</>
	);
};

export const MediaDetails = (props: { media: MediaT }): JSX.Element => {
	const api = useApi();
	const uploader = api.users.use(() => props.media.user_id ?? undefined);

	return (
		<div class="media-page-details">
			<div class="field">
				<div class="dim">status</div>
				<div>{props.media.status}</div>
			</div>
			<div class="field">
				<div class="dim">filename</div>
				<Copyable2 name="filename">{props.media.filename}</Copyable2>
			</div>
			<Show when={props.media.alt}>
				{(alt) => (
					<div class="field">
						<div class="dim">alt</div>
						<Copyable2 name="alt text">{alt()}</Copyable2>
					</div>
				)}
			</Show>
			<Show when={props.media.user_id}>
				<div class="field">
					<div class="dim">uploaded by</div>
					<Show when={uploader()} fallback={<div>loading...</div>}>
						{(u) => (
							<div class="user">
								<Avatar user={u()} />
								<UserDisplayName user_id={u().id} onClick />
							</div>
						)}
					</Show>
				</div>
			</Show>
			<div class="field">
				<div class="dim">scans</div>
				<For
					each={props.media.scans ?? []}
					fallback={<div class="missing">no scans</div>}
				>
					{(scan) => (
						<div>
							{scan.key}
							<span class="dim-1em">:</span> {(scan.result * 100).toFixed(2)}%{" "}
							<span class="dim-1em">(version {scan.version})</span>
						</div>
					)}
				</For>
			</div>
			<div class="field">
				<div class="dim">links</div>
				<For
					each={props.media.links ?? []}
					fallback={<div class="missing">no scans</div>}
				>
					{(link) => (
						<div>
							<MediaLink link={link} />
						</div>
					)}
				</For>
			</div>
		</div>
	);
};

// TODO: impl and use this
const MediaLink = (props: { link: MediaLinkType }) => {
	const renderLinkType = (link: MediaLinkType) => {
		switch (link.type) {
			case "Message":
				return `${link.channel_id} ${link.message_id}`;
			case "MessageVersion":
				return `${link.channel_id} ${link.message_id}`;
			case "UserAvatar":
				return `${link.user_id}`;
			case "UserBanner":
				return `${link.user_id}`;
			case "ChannelIcon":
				return `${link.channel_id}`;
			case "RoomIcon":
				return `${link.room_id}`;
			case "RoomBanner":
				return `${link.room_id}`;
			case "Embed":
				return `${link.id}`;
			case "CustomEmoji":
				return `${link.room_id}`;
			case "Script":
				return `${link.channel_id} ${link.script_id}`;
			case "ScriptVersion":
				return `${link.channel_id} ${link.script_id} ${link.version_id}`;
			case "Document":
				return `${link.channel_id} ${link.document_id}`;
			default:
				return `unknown link type ${link.type}`; // TODO: make typescript happy
		}
	};

	return <>{JSON.stringify(props.link, null, 2)}</>;
};
