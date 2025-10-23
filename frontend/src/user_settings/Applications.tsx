import {
	createEffect,
	createResource,
	createSignal,
	For,
	onCleanup,
	Show,
	type VoidProps,
} from "solid-js";
import { type Application, type User } from "sdk";
import { useApi } from "../api.tsx";
import { Copyable } from "../util.tsx";
import { createStore, reconcile } from "solid-js/store";
import { useCtx } from "../context.ts";
import { useFloating } from "solid-floating-ui";
import { ReferenceElement, shift } from "@floating-ui/dom";
import { usePermissions } from "../hooks/usePermissions.ts";

const SessionList = (props: { appId: string }) => {
	const api = useApi();
	const ctx = useCtx();

	const [sessions, { refetch }] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/session", {
			headers: { "x-puppet-id": props.appId },
		});
		return data?.items ?? [];
	});

	const revokeSession = (sessionId: string) => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "Are you sure you want to revoke this session?",
			cont: async (confirmed) => {
				if (confirmed) {
					await api.client.http.DELETE("/api/v1/session/{session_id}", {
						params: { path: { session_id: sessionId } },
					});
					refetch();
				}
			},
		});
	};

	const renameSession = (sessionId: string) => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "New session name?",
			cont: async (name) => {
				if (name === null) return;
				await api.client.http.PATCH("/api/v1/session/{session_id}", {
					params: { path: { session_id: sessionId } },
					body: { name: name || null },
				});
				refetch();
			},
		});
	};

	return (
		<div class="sessions-list">
			<h4>Sessions</h4>
			<Show when={sessions.loading}>Loading sessions...</Show>
			<Show when={sessions() && sessions()!.length > 0}>
				<ul>
					<For each={sessions()}>
						{(session) => (
							<li>
								<span>{session.name || session.id}</span>
								<button onClick={() => renameSession(session.id)}>
									Rename
								</button>
								<button onClick={() => revokeSession(session.id)}>
									Revoke
								</button>
							</li>
						)}
					</For>
				</ul>
			</Show>
			<Show when={sessions() && sessions()!.length === 0}>
				<p>No active sessions.</p>
			</Show>
		</div>
	);
};

// TODO: in create session and rotate oauth token, make the secret Copyable
export function Applications(_props: VoidProps<{ user: User }>) {
	const api = useApi();

	async function create() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "New app name?",
			cont: async (name) => {
				if (!name) return;
				await api.client.http.POST("/api/v1/app", {
					body: {
						name,
						bridge: false,
						public: false,
					},
				});
				refetch();
			},
		});
	}

	const [list, { refetch }] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/app", {
			params: { query: { limit: 100 } },
		});
		return data;
	});

	const [apps, setApps] = createStore<Application[]>([]);
	const [originalApps, setOriginalApps] = createSignal<Application[]>([]);

	createEffect(() => {
		if (list()) {
			setOriginalApps(JSON.parse(JSON.stringify(list()!.items)));
			setApps(reconcile(list()!.items));
		}
	});

	const hasUnsavedChanges = () => {
		const orig = originalApps();
		if (orig.length === 0 && apps.length === 0) return false;
		if (orig.length !== apps.length) return true;
		return JSON.stringify(orig) !== JSON.stringify(apps);
	};

	const cancelChanges = () => {
		setApps(reconcile(originalApps()));
	};

	const saveChanges = async () => {
		const promises = apps
			.map((app) => {
				const originalApp = originalApps().find((o) => o.id === app.id);
				if (
					originalApp && JSON.stringify(app) !== JSON.stringify(originalApp)
				) {
					return api.client.http.PATCH("/api/v1/app/{app_id}", {
						params: { path: { app_id: app.id } },
						body: {
							name: app.name,
							description: app.description,
							bridge: app.bridge,
							public: app.public,
							oauth_confidential: app.oauth_confidential,
							oauth_redirect_uris: app.oauth_redirect_uris,
						},
					});
				}
				return null;
			})
			.filter(Boolean);

		if (promises.length > 0) {
			await Promise.all(promises);
			setOriginalApps(JSON.parse(JSON.stringify(apps)));
			refetch();
		}
	};

	const ctx = useCtx();
	const rotateSecret = async (app_id: string) => {
		const { data } = await api.client.http.POST(
			"/api/v1/app/{app_id}/rotate-secret",
			{
				params: { path: { app_id } },
			},
		);
		ctx.dispatch({
			do: "modal.alert",
			text: `your secret is ${data?.oauth_secret} (this can only be seen once)`,
		});
	};

	const [inviteApp, setInviteApp] = createSignal<
		{ app: Application; x: number; y: number }
	>();
	const InviteAppClear = () => setInviteApp();
	document.addEventListener("click", InviteAppClear);
	onCleanup(() => document.removeEventListener("click", InviteAppClear));

	const createSession = async (app_id: string) => {
		const { data } = await api.client.http.POST(
			"/api/v1/app/{app_id}/session",
			{
				params: { path: { app_id } },
				body: { name: "session" },
			},
		);
		ctx.dispatch({
			do: "modal.alert",
			text: `your secret is ${data?.token} (this can only be seen once)`,
		});
	};

	const [search, setSearch] = createSignal("");
	// TODO: use fuzzysort here

	const updateApp = (index: number, field: keyof Application, value: any) => {
		setApps(index, field, value);
	};

	return (
		<div class="applications-settings">
			<h2>applications</h2>
			<header class="applications-header">
				<input
					type="search"
					placeholder="search"
					aria-label="search"
					onInput={(e) => setSearch(e.target.value)}
				/>
				<button type="button" class="primary big" onClick={create}>
					create
				</button>
			</header>
			<ul class="applications-list">
				<For each={apps.filter((i) => i.name.includes(search()))}>
					{(app, index) => {
						return (
							<li>
								<h3 class="dim">name</h3>
								<input
									type="text"
									value={app.name}
									onInput={(e) =>
										updateApp(index(), "name", e.currentTarget.value)}
								/>
								<div style="height: 8px" />
								<h3 class="dim">description</h3>
								<textarea
									onInput={(e) =>
										updateApp(index(), "description", e.currentTarget.value)}
								>
									{app.description ?? ""}
								</textarea>
								<div style="height: 8px" />
								<h3 class="dim">id (click to copy)</h3>
								<Copyable>{app.id}</Copyable>
								<div style="height: 8px" />
								<label>
									<input
										type="checkbox"
										checked={app.bridge}
										onInput={(e) =>
											updateApp(index(), "bridge", e.currentTarget.checked)}
									/>{" "}
									bridge
								</label>
								<br />
								<label>
									<input
										type="checkbox"
										checked={app.public}
										onInput={(e) =>
											updateApp(index(), "public", e.currentTarget.checked)}
									/>{" "}
									public
								</label>
								<br />
								<br />
								<div class="oauth">
									<b>oauth settings</b>
									<br />
									<label>
										<input
											type="checkbox"
											checked={app.oauth_confidential}
											onInput={(e) =>
												updateApp(
													index(),
													"oauth_confidential",
													e.currentTarget.checked,
												)}
										/>
									</label>{" "}
									confidential (can keep secrets)
									<br />
									<h3 class="dim">redirect uris</h3>
									<ul>
										<For each={app.oauth_redirect_uris}>
											{(uri, uriIndex) => (
												<li>
													<input
														type="text"
														value={uri}
														onInput={(e) => {
															const newUris = [
																...app.oauth_redirect_uris ?? [],
															];
															newUris[uriIndex()] = e.currentTarget.value;
															updateApp(
																index(),
																"oauth_redirect_uris",
																newUris,
															);
														}}
													/>
													<button
														onClick={() => {
															const newUris = [
																...app.oauth_redirect_uris ?? [],
															];
															newUris.splice(uriIndex(), 1);
															updateApp(
																index(),
																"oauth_redirect_uris",
																newUris,
															);
														}}
													>
														remove
													</button>
												</li>
											)}
										</For>
										<li>
											<button
												onClick={() => {
													const newUris = [
														...app.oauth_redirect_uris ?? [],
														"",
													];
													updateApp(index(), "oauth_redirect_uris", newUris);
												}}
											>
												add uri
											</button>
										</li>
									</ul>
									<br />
									<button onClick={() => rotateSecret(app.id)}>
										rotate secret
									</button>
								</div>
								<div class="sessions">
									<button onClick={() => createSession(app.id)}>
										create session
									</button>
									<SessionList appId={app.id} />
								</div>
								<div class="invite">
									<button
										onClick={(e) => {
											e.stopImmediatePropagation();
											setInviteApp({
												app,
												x: e.clientX,
												y: e.clientY,
											});
										}}
									>
										add to room
									</button>
								</div>
							</li>
						);
					}}
				</For>
			</ul>
			<Show when={hasUnsavedChanges()}>
				<div class="applications-savebar">
					<div class="applications-savebar-inner">
						<div class="warning">you have unsaved changes</div>
						<button class="reset" onClick={cancelChanges}>
							cancel
						</button>
						<button class="save" onClick={saveChanges}>
							save
						</button>
					</div>
				</div>
			</Show>
			<Show when={inviteApp()}>
				{(app) => (
					<InviteToRoom
						x={app().x}
						y={app().y}
						app={app().app}
					/>
				)}
			</Show>
		</div>
	);
}

// TODO: make this an actual context menu?
const InviteToRoom = (
	props: { x: number; y: number; app: Application },
) => {
	const api = useApi();
	const rooms = api.rooms.list();
	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();

	createEffect(() => {
		setMenuParentRef({
			getBoundingClientRect: () => ({
				x: props.x,
				y: props.y,
				left: props.x,
				top: props.y,
				right: props.x,
				bottom: props.y,
				width: 0,
				height: 0,
			}),
		});

		props.x;
		props.y;
	});

	const menuFloating = useFloating(() => menuParentRef(), () => menuRef(), {
		middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
		placement: "right-start",
	});

	const inviteToRoom = (room_id: string) => {
		api.client.http.POST("/api/v1/app/{app_id}/invite", {
			params: { path: { app_id: props.app.id } },
			body: { room_id },
		});
	};

	const self_id = () => api.users.cache.get("@self")!.id;

	return (
		<menu
			class="invite-app"
			style={{
				translate: `${menuFloating.x}px ${menuFloating.y}px`,
			}}
			ref={setMenuRef}
		>
			<For each={rooms()?.items ?? []} fallback="no rooms?">
				{(r) => {
					const perms = usePermissions(
						self_id,
						() => r.id,
						() => undefined,
					);

					return (
						<button
							onClick={[inviteToRoom, r.id]}
							disabled={!perms.has("IntegrationsManage")}
						>
							{r.name}
						</button>
					);
				}}
			</For>
		</menu>
	);
};
