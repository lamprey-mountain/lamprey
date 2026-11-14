import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import {
	createEffect,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { useInvites, useRoomMembers, useUsers } from "@/api";
import { Time } from "@/atoms/Time.tsx";
import { Avatar } from "@/components/shared/User";
import { useModals } from "@/contexts/modal";
import type { RoomT } from "@/types";
import { Copyable, getDate } from "@/utils/general";

export function Invites(props: VoidProps<{ room: RoomT }>) {
	const invites2 = useInvites();
	const users2 = useUsers();
	const roomMembers2 = useRoomMembers();
	const [, modalCtl] = useModals();

	const invites = invites2.useRoomList(() => props.room.id);

	const createInvite = () => {
		modalCtl.open({ type: "invite_create", room_id: props.room.id });
	};

	const deleteInvite = (code: string) => {
		modalCtl.confirm(
			"are you sure you want to delete this invite?",
			async (confirmed) => {
				if (!confirmed) return;
				try {
					await invites2.deleteInvite(code);
				} catch (e) {
					console.error("failed to delete invite", e);
					modalCtl.alert("failed to delete invite");
				}
			},
		);
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	const fetchMore = () => {
		const list = invites();
		if (!list || !list.state.has_more || list.state.isLoading) return;
		invites2.fetchNextRoomPage(props.room.id);
	};

	// keep fetching while the sentinel is visible (e.g. when a page is too
	// short to push the sentinel out of the viewport)
	createEffect(() => {
		const list = invites();
		const el = bottom();
		if (!list || !el || !el.isConnected) return;
		if (!list.state.has_more || list.state.isLoading) return;
		if (el.getBoundingClientRect().top > window.innerHeight) return;
		fetchMore();
	});

	createIntersectionObserver(
		() => (bottom() ? [bottom()!] : []),
		(entries) => {
			if (entries.some((entry) => entry.isIntersecting)) fetchMore();
		},
	);

	return (
		<>
			<h2>invites</h2>
			<button type="button" class="button big primary" onClick={createInvite}>
				create invite
			</button>
			<br />
			<div class="invites">
				<header>
					<div class="code">code</div>
					<div class="creator">creator</div>
					<div class="uses">uses</div>
					<div class="expires">expires</div>
				</header>
				<Show
					when={invites()}
					fallback={
						<div class="dim empty">
							{invites.error ? "failed to load invites" : "loading..."}
						</div>
					}
				>
					{(list) => (
						<Show
							when={list().state.ids.length > 0}
							fallback={<div class="dim empty">no invites yet</div>}
						>
							<ul>
								<For each={list().state.ids}>
									{(code) => {
										const i = invites2.cache.get(code);
										if (!i) return null;
										const user = users2.cache.get(i.creator_id);
										const rm = roomMembers2.cache.get(
											`${props.room.id}:${i.creator_id}`,
										);
										const creatorName = () => {
											return rm?.override_name || user?.name || "unknown";
										};
										const expired = () =>
											!!i.expires_at &&
											getDate(i.expires_at).valueOf() < Date.now();
										return (
											<li class="invites-invite">
												<div class="code">
													<Copyable>{i.code}</Copyable>
													<Show when={i.description}>
														{(desc) => <div class="dim">{desc()}</div>}
													</Show>
												</div>
												<div class="creator">
													<Avatar user={i.creator} pad={0} />
													<div class="info">
														<div class="name">{creatorName()}</div>
														<Time date={getDate(i.created_at)} />
													</div>
												</div>
												<div class="uses">
													<span class="mono">{i.uses ?? 0}</span>
													<span class="dim">/</span>
													<span class="mono">{i.max_uses ?? "\u221e"}</span>
												</div>
												<div class="expires">
													<Show
														when={i.expires_at}
														fallback={<span class="dim">never</span>}
													>
														<Show
															when={!expired()}
															fallback={<span class="expired">expired</span>}
														>
															<Time date={getDate(i.expires_at!)} />
														</Show>
													</Show>
												</div>
												<div>
													<Show when={!i.is_vanity}>
														<button
															type="button"
															class="button"
															onClick={() => deleteInvite(i.code)}
														>
															delete
														</button>
													</Show>
												</div>
											</li>
										);
									}}
								</For>
							</ul>
							<div ref={setBottom}></div>
						</Show>
					)}
				</Show>
			</div>
		</>
	);
}
