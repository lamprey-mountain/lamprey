import { createResource, For, Show, VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { Pagination, SessionT, UserT } from "../types.ts";

export function Sessions(props: VoidProps<{ user: UserT }>) {
	const ctx = useCtx();
	const api = useApi();

	// FIXME: live update sessions
	const [sessions, { refetch: fetchSessions }] = createResource<
		Pagination<SessionT> & { user_id: string },
		string
	>(() => props.user.id, async (user_id, { value }) => {
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
	});

	function renameSession(session_id: string) {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.PATCH("/api/v1/session/{session_id}", {
					params: { path: { session_id } },
					body: { name },
				});
			},
		});
	}

	function revokeSession(session_id: string) {
		ctx.dispatch({
			do: "modal.confirm",
			text: "really delete this session?",
			cont(confirmed) {
				if (!confirmed) return;
				ctx.client.http.DELETE("/api/v1/session/{session_id}", {
					params: { path: { session_id } },
				});
			},
		});
	}

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
									<button onClick={() => renameSession(s.id)}>rename</button>
									<button onClick={() => revokeSession(s.id)}>revoke</button>
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
				<button onClick={fetchSessions}>more</button>
			</Show>
		</>
	);
}
