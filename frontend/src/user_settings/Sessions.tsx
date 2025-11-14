import { createResource, For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { useModals } from "../contexts/modal";
import type { Pagination, SessionT, UserT } from "../types.ts";
import { ResourceFetcherInfo } from "solid-js";

export function Sessions(props: VoidProps<{ user: UserT }>) {
	const ctx = useCtx();
	const api = useApi();
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

	// TODO: redo styles

	return (
		<>
			<h2>sessions</h2>
			<Show when={sessions()}>
				<ul>
					<For each={sessions()!.items}>
						{(s) => (
							<li>
								<div>
									<Show when={s.name} fallback={<em>no name</em>}>
										{s.name}
									</Show>{" "}
									- {s.status}
									<button type="button" onClick={() => renameSession(s.id)}>
										rename
									</button>
									<button type="button" onClick={() => revokeSession(s.id)}>
										revoke
									</button>
								</div>
								<div>
									<code class="dim">{s.id}</code>
									<Show when={s.id === api.session()?.id}>
										{" (current)"}
									</Show>
								</div>
							</li>
						)}
					</For>
				</ul>
				<button type="button" onClick={fetchSessions}>more</button>
			</Show>
		</>
	);
}
