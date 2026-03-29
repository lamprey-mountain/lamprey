import { createResource, For, Show, type VoidProps } from "solid-js";
import { useApi2 } from "@/api";
import { useCtx } from "../../../context.ts";
import { useModals } from "../../../contexts/modal";
import type { Pagination, SessionT, UserT } from "../../../types.ts";
import type { ResourceFetcherInfo } from "solid-js";
import { Time } from "../../../atoms/Time.tsx";
import { Copyable } from "../../../utils/general";

function parseUA(ua: string) {
	if (/iPhone|iPad/.test(ua)) return { icon: "mobile", label: "iOS" };
	if (/Android/.test(ua)) return { icon: "mobile", label: "Android" };
	if (/Mac/.test(ua)) return { icon: "desktop", label: "macOS" };
	if (/Windows/.test(ua)) return { icon: "desktop", label: "Windows" };
	if (/Linux/.test(ua)) return { icon: "desktop", label: "Linux" };
	return { icon: "desktop", label: "Unknown" };
}

export function Sessions(props: VoidProps<{ user: UserT }>) {
	const ctx = useCtx();
	const api2 = useApi2();
	const [, modalctl] = useModals();

	// FIXME: live update sessions
	const [sessions, { refetch: fetchSessions }] = createResource<
		Pagination<SessionT> & { user_id: string },
		string
	>(
		() => props.user.id,
		async (
			user_id: string,
			{ value }: ResourceFetcherInfo<
				Pagination<SessionT> & { user_id: string }
			>,
		) => {
			if (value?.user_id !== user_id) value = undefined;
			if (value?.has_more === false) return value;
			const lastId = value?.items.at(-1)?.id ??
				"00000000-0000-0000-0000-000000000000";
			const { data, error } = await ctx.client.http.GET("/api/v1/session", {
				params: {
					query: {
						from: lastId,
						limit: 100,
						dir: "f",
					},
				},
			});
			if (error) throw new Error(error);
			return {
				...data,
				items: [...value?.items ?? [], ...data.items],
				user_id,
			};
		},
	);

	function renameSession(session_id: string) {
		modalctl.prompt("name?", (name: string | null) => {
			if (!name) return;
			ctx.client.http.PATCH("/api/v1/session/{session_id}", {
				params: { path: { session_id } },
				body: { name },
			});
		});
	}

	function revokeSession(session_id: string) {
		modalctl.confirm("really delete this session?", (confirmed: boolean) => {
			if (!confirmed) return;
			ctx.client.http.DELETE("/api/v1/session/{session_id}", {
				params: { path: { session_id } },
			});
		});
	}

	// TODO: order by id (created at), last_seen_at

	const isSudoActive = (s: SessionT) => {
		if (s.status !== "Sudo") return false;
		if (!s.sudo_expires_at) return false;
		return new Date(s.sudo_expires_at) > new Date();
	};

	return (
		<div class="user-settings-sessions">
			<h2>sessions</h2>
			<Show when={sessions()}>
				<ul>
					<For each={sessions()!.items}>
						{(s) => (
							<li
								class="session"
								classList={{
									current: s.id === api2.session()?.id,
									sudo: isSudoActive(s),
								}}
							>
								<div class="info">
									<div>
										<Show when={s.name} fallback={<em>no name</em>}>
											{s.name}
										</Show>
									</div>
									<menu>
										<button type="button" onClick={() => renameSession(s.id)}>
											rename
										</button>
										<button type="button" onClick={() => revokeSession(s.id)}>
											revoke
										</button>
									</menu>
								</div>
								<div class="meta">
									<Time date={new Date(s.last_seen_at)} />
									<span class="bullet"></span>
									{s.user_agent
										? parseUA(s.user_agent).label
										: <span class="unknown">unknown ua</span>}
									<span class="bullet"></span>
									{s.ip_addr ?? <span class="unknown">unknown ip</span>}
								</div>
								<div class="dim">
									<Copyable>{s.id}</Copyable>
									<Show when={s.id === api2.session()?.id}>
										{" (current)"}
									</Show>
								</div>
							</li>
						)}
					</For>
				</ul>
				<button type="button" onClick={fetchSessions}>more</button>
			</Show>
		</div>
	);
}
