import { For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import type { Channel } from "sdk";
import type { InviteWithMetadata } from "sdk";
import { Avatar } from "../User.tsx";
import { Time } from "../Time.tsx";
import { Copyable } from "../util.tsx";

export function Invites(props: VoidProps<{ channel: Channel }>) {
	const api = useApi();

	const invites = api.channel_invites.list(() => props.channel.id);

	const createInvite = () => {
		api.client.http.POST("/api/v1/channel/{channel_id}/invite", {
			params: {
				path: { channel_id: props.channel.id },
			},
			body: {},
		});
	};

	const deleteInvite = (code: string) => {
		api.client.http.DELETE("/api/v1/invite/{invite_code}", {
			params: {
				path: { invite_code: code },
			},
		});
	};

	return (
		<>
			<h2>invites</h2>
			<button class="big primary" onClick={createInvite}>create invite</button>
			<br />
			<br />
			<div class="invites">
				<Show when={!invites.loading} fallback="loading...">
					<header>
						<div class="code">code</div>
						<div class="creator">creator</div>
						<div class="uses">uses</div>
						<div class="expires">expires</div>
					</header>
					<ul>
						<For each={invites()!.items as InviteWithMetadata[]}>
							{(i) => {
								const user = api.users.fetch(() => i.creator_id);
								const creatorName = () => user()?.name || "unknown";
								return (
									<li class="invite">
										<div class="code">
											<Copyable>{i.code}</Copyable>
										</div>
										<div class="creator">
											<Avatar user={i.creator} pad={0} />
											<div class="info">
												<div class="name">{creatorName()}</div>
												<Time date={new Date(i.created_at)} />
											</div>
										</div>
										<div class="uses">
											<span class="mono">{i.uses}</span>
											<span class="dim">/</span>
											<span class="mono">{i.max_uses ?? "\u221e"}</span>
										</div>
										<div class="expires">
											<Show
												when={i.expires_at}
												fallback={<span class="dim">never</span>}
											>
												<Time date={new Date(i.expires_at!)} />
											</Show>
										</div>
										<div>
											<button onClick={() => deleteInvite(i.code)}>
												delete
											</button>
										</div>
									</li>
								);
							}}
						</For>
					</ul>
				</Show>
			</div>
		</>
	);
}
