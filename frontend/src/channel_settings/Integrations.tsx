import {
	createEffect,
	createResource,
	createSignal,
	For,
	onCleanup,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import type { Channel } from "sdk";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { Avatar } from "../User.tsx";
import { Time } from "../Time.tsx";
import { useFloating } from "solid-floating-ui";
import { ReferenceElement, shift } from "@floating-ui/dom";
import { usePermissions } from "../hooks/usePermissions.ts";
import { getTimestampFromUUID, RoomMemberOrigin } from "sdk";

export function Integrations(props: VoidProps<{ channel: Channel }>) {
	const ctx = useCtx();
	const api = useApi();

	const editRolesClear = () => setEditRoles();
	document.addEventListener("click", editRolesClear);
	onCleanup(() => document.removeEventListener("click", editRolesClear));

	const [integrations] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/channel/{channel_id}/webhook",
			{ params: { path: { channel_id: props.channel.id } } },
		);
		return data;
	});

	const removeWebhook = (webhook_id: string) => () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "really remove?",
			cont(conf) {
				if (!conf) return;
				api.client.http.DELETE(
					"/api/v1/webhook/{webhook_id}",
					{ params: { path: { webhook_id } } },
				);
			},
		});
	};

	const fetchMore = () => {
		// TODO
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(() => bottom() ? [bottom()!] : [], (entries) => {
		for (const entry of entries) {
			if (entry.isIntersecting) fetchMore();
		}
	});

	const [editRoles, setEditRoles] = createSignal<any>(); // TODO: type this

	return (
		<div class="room-settings-members">
			<h2>integrations</h2>
			<header>
				<div class="name">name</div>
				<div class="joined">created</div>
			</header>
			<Show when={integrations()}>
				<ul>
					<For each={integrations()!.items}>
						{(i) => {
							const user = api.users.fetch(() => i.creator_id);
							const name = () => i.name;
							return (
								<li>
									<div class="profile">
										<Avatar user={user()} />
										<div>
											<h3 class="name">{name()}</h3>
										</div>
									</div>
									<div class="joined">
										<Time date={getTimestampFromUUID(i.id)} />
									</div>
									<div style="flex:1"></div>
									<button
										onClick={removeWebhook(i.id)}
									>
										delete
									</button>
								</li>
							);
						}}
					</For>
				</ul>
				<div ref={setBottom}></div>
			</Show>
		</div>
	);
}
