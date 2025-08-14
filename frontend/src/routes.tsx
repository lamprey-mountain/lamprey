import { A, RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { flags } from "./flags.ts";
import { ChatNav } from "./Nav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { Accessor, createEffect, For, Show } from "solid-js";
import { RoomSettings } from "./RoomSettings.tsx";
import { ThreadSettings } from "./ThreadSettings.tsx";
import { ChatHeader, ChatMain } from "./Chat.tsx";
import { ThreadMembers } from "./Thread.tsx";
import { Home } from "./Home.tsx";
import { Voice } from "./Voice.tsx";
import { Feed } from "./Feed.tsx";
import { getUrl } from "./media/util.tsx";

const Title = (props: { title?: string }) => {
	createEffect(() => document.title = props.title ?? "");
	return undefined;
};

export const Nav2 = () => {
	const api = useApi();
	const rooms = api.rooms.list();

	function getThumb(media_id: string) {
		const media = api.media.fetchInfo(() => media_id);
		const m = media();
		if (!m) return;
		const tracks = [m.source, ...m.tracks];
		const source =
			tracks.find((s) => s.type === "Thumbnail" && s.height === 64) ??
				tracks.find((s) => s.type === "Image");
		if (source) {
			return getUrl(source);
		} else {
			console.error("no valid avatar source?", m);
		}
	}

	return (
		<Show when={flags.has("two_tier_nav")}>
			<nav class="nav2">
				<ul>
					<li>
						<A href="/" end>home</A>
					</li>
					<For each={rooms()?.items}>
						{(room) => (
							<li draggable="true" class="menu-room" data-room-id={room.id}>
								<A draggable="false" href={`/room/${room.id}`} class="nav">
									<Show
										when={room.icon}
										fallback={<div class="avatar">{room.name}</div>}
									>
										<img
											src={getThumb(room.icon!)}
											class="avatar"
										/>
									</Show>
								</A>
							</li>
						)}
					</For>
				</ul>
			</nav>
		</Show>
	);
};

export const RouteRoom = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	return (
		<>
			<Title title={room() ? room()!.name : t("loading")} />
			<Nav2 />
			<ChatNav room_id={p.params.room_id} />
			<Show when={room()}>
				<RoomHome room={room()!} />
				<Show when={flags.has("room_member_list")}>
					<RoomMembers room={room()!} />
				</Show>
			</Show>
		</>
	);
};

export const RouteRoomSettings = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	const title = () =>
		room() ? t("page.settings_room", room()!.name) : t("loading");
	return (
		<>
			<Title title={title()} />
			<Show when={room()}>
				<RoomSettings room={room()!} page={p.params.page} />
			</Show>
		</>
	);
};

export const RouteThreadSettings = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const title = () =>
		thread() ? t("page.settings_thread", thread()!.name) : t("loading");
	return (
		<>
			<Title title={title()} />
			<Show when={thread()}>
				<ThreadSettings thread={thread()!} page={p.params.page} />
			</Show>
		</>
	);
};

export const RouteThread = (p: RouteSectionProps) => {
	const { t } = useCtx();
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const room = api.rooms.fetch(() => thread()?.room_id!);

	// fetch threads to populate sidebar
	api.threads.list(() => thread()?.room_id!);

	const title = () => {
		if (!thread()) return t("loading");
		return room() && thread()?.room_id
			? `${thread()!.name} - ${room()!.name}`
			: thread()!.name;
	};

	return (
		<>
			<Title title={title()} />
			<Nav2 />
			<ChatNav room_id={thread()?.room_id} />
			<Show when={thread()}>
				<ChatHeader thread={thread()!} />
				<Show
					when={thread().type === "Chat" || thread().type === "Dm" ||
						thread().type === "Gdm"}
				>
					<ChatMain thread={thread()!} />
				</Show>
				<Show when={thread().type === "Voice"}>
					<Voice thread={thread()!} />
				</Show>
				<Show when={flags.has("thread_member_list")}>
					<ThreadMembers thread={thread()!} />
				</Show>
			</Show>
		</>
	);
};

export const RouteHome = () => {
	const { t } = useCtx();
	return (
		<>
			<Title title={t("page.home")} />
			<Nav2 />
			<ChatNav />
			<Home />
		</>
	);
};

export const RouteFeed = () => {
	return (
		<>
			<Title title="feed" />
			<Nav2 />
			<Feed />
		</>
	);
};
