import type { Channel, InviteWithMetadata } from "sdk";
import { For, Show, type VoidProps } from "solid-js";
import { useApi2, useInvites2, useUsers2 } from "@/api";
import { Time } from "../../../atoms/Time.tsx";
import { Avatar } from "../../../User.tsx";
import { Copyable } from "../../../utils/general";

export function Invites(props: VoidProps<{ channel: Channel }>) {
	const api2 = useApi2();
	const invites2 = useInvites2();
	const users2 = useUsers2();

	const invites = invites2.useChannelList(() => props.channel.id);

	const createInvite = () => {
		api2.client.http.POST("/api/v1/channel/{channel_id}/invite", {
			params: {
				path: { channel_id: props.channel.id },
			},
			body: {},
		});
	};

	const deleteInvite = (code: string) => {
		api2.client.http.DELETE("/api/v1/invite/{invite_code}", {
			params: {
				path: { invite_code: code },
			},
		});
	};

	return (
		<>
			<h2>invites</h2>
			<button class="big primary" onClick={createInvite}>
				create invite
			</button>
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
						<For each={invites()!.state.ids}>
							{(code) => {
								const i = invites2.cache.get(code);
								if (!i) return null;
								const user = users2.use(() => i.creator_id);
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
											<span class="mono">{(i as any).uses}</span>
											<span class="dim">/</span>
											<span class="mono">
												{(i as any).max_uses ?? "\u221e"}
											</span>
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
