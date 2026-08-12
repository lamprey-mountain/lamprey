import { createResource, For, Match, Show, Switch } from "solid-js";
import { useApi } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Time } from "@/atoms/Time";
import { createTooltip } from "@/atoms/Tooltip";
import { useModals2 } from "@/contexts/modal";
import { icDelete, icEdit } from "@/utils/icons";
import type { ApplicationDraft } from "./context";

// TODO: deduplicate this code with frontend/src/components/features/user_settings/Sessions.tsx

export const Sessions = (props: { draft: ApplicationDraft }) => {
	const api = useApi();
	const modals = useModals2();

	const isCreate = () => props.draft.state === "create";
	const appId = () => (isCreate() ? props.draft.nonce : props.draft.data.id);

	const [sessions, { refetch }] = createResource(
		() => appId(),
		async (id) => {
			if (isCreate()) return [];
			const { data } = await api.client.http.GET("/api/v1/session", {
				headers: { "x-puppet-id": id },
			});
			return data?.items ?? [];
		},
	);

	const revokeSession = (sessionId: string) => {
		modals
			.confirm("Are you sure you want to revoke this session?")
			.then(async (confirmed) => {
				if (confirmed) {
					await api.client.http.DELETE("/api/v1/session/{session_id}", {
						params: { path: { session_id: sessionId } },
					});
					refetch();
				}
			});
	};

	const renameSession = (sessionId: string) => {
		modals.prompt("New session name?").then(async (name) => {
			if (name === null) return;
			await api.client.http.PATCH("/api/v1/session/{session_id}", {
				params: { path: { session_id: sessionId } },
				body: { name: name || null },
			});
			refetch();
		});
	};

	return (
		<div class="sessions">
			<h3>sessions</h3>
			<Show when={!isCreate()}>
				<button
					type="button"
					class="button"
					onClick={async () => {
						const { data } = await api.client.http.POST(
							"/api/v1/app/{app_id}/session",
							{
								params: { path: { app_id: appId() } },
								body: { name: "session" },
							},
						);
						modals.alert(
							`your secret is ${data?.token} (this can only be seen once)`,
						);
						refetch();
					}}
				>
					create session
				</button>
			</Show>
			<div class="sessions-list">
				<h4 class="dim" style="margin-top:8px">
					<Switch>
						<Match when={sessions.loading}>Loading sessions...</Match>
						<Match when={sessions().length === 0}>No sessions.</Match>
						<Match when={sessions().length === 1}>1 session.</Match>
						<Match when={true}>{sessions().length} sessions.</Match>
					</Switch>
				</h4>
				<Show when={sessions()?.length}>
					<ul>
						<For each={sessions()}>
							{(session) => {
								const tipRename = createTooltip({ tip: () => "Rename" });
								const tipRevoke = createTooltip({ tip: () => "Revoke" });

								return (
									<li>
										<div class="session">
											<div class="info">
												<div class="name">{session.name || session.id}</div>
												<menu>
													<button
														type="button"
														class="button icon-button"
														onClick={[renameSession, session.id]}
														ref={tipRename.content}
													>
														<Icon src={icEdit} />
													</button>
													<button
														type="button"
														class="button icon-button danger"
														onClick={[revokeSession, session.id]}
														ref={tipRevoke.content}
													>
														<Icon src={icDelete} />
													</button>
												</menu>
											</div>
											<div class="meta">
												<Time date={new Date(session.imprint.last_seen_at)} />
											</div>
										</div>
									</li>
								);
							}}
						</For>
					</ul>
				</Show>
				<Show when={sessions() && sessions()?.length === 0}>
					<p>No active sessions.</p>
				</Show>
			</div>
		</div>
	);
};
