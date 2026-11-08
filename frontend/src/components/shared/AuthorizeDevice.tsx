import type { RouteSectionProps } from "@solidjs/router";
import {
	createResource,
	createSignal,
	ErrorBoundary,
	For,
	type JSX,
	Show,
} from "solid-js";
import { useCtx } from "@/app/context";
import { getScopeDescription } from "@/components/shared/Oauth";

type DeviceInfo = {
	application: { name: string; description?: string | null };
	auth_user: { name: string };
	scopes: string[];
};

export const RouteDevice = (_p: RouteSectionProps): JSX.Element => {
	const query = URL.parse(location.href)?.searchParams.get("user_code") ?? "";
	const [userCode, setUserCode] = createSignal(query);
	const [draftCode, setDraftCode] = createSignal(query);
	const ctx = useCtx();
	const [busy, setBusy] = createSignal(false);
	const [done, setDone] = createSignal<"approved" | "denied" | null>(null);
	const [error, setError] = createSignal<string | null>(null);

	const [data] = createResource(userCode, async (code) => {
		if (!code) return;
		const { data, error } = await ctx.client.http.GET(
			`/api/v1/oauth/device?user_code=${encodeURIComponent(code)}` as any,
			{} as any,
		);
		if (error) throw (error as any).error ?? (error as any).message;
		return data as DeviceInfo;
	});

	const decide = async (approve: boolean) => {
		try {
			setBusy(true);
			setError(null);
			const { error } = await ctx.client.http.POST(
				"/api/v1/oauth/device/authorize" as any,
				{
					body: { user_code: userCode(), approve },
				} as any,
			);
			if (error) {
				setError((error as any)?.message ?? "could not authorize");
			} else {
				setDone(approve ? "approved" : "denied");
			}
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(false);
		}
	};

	return (
		<div class="oauth-authorize">
			<Show when={!userCode()}>
				<form
					onSubmit={(event) => {
						event.preventDefault();
						setUserCode(draftCode().trim());
					}}
				>
					<h2>Authorize a device</h2>
					<p>Enter the code shown on your device.</p>
					<input
						value={draftCode()}
						onInput={(event) => setDraftCode(event.currentTarget.value)}
						type="text"
						autocomplete="off"
						autocapitalize="characters"
						placeholder="XXXX-XXXX"
						autofocus
						class="code-input"
					/>
					<menu>
						<button type="submit" class="button big primary">
							continue
						</button>
					</menu>
				</form>
			</Show>
			<Show when={userCode()}>
				<Show when={done()}>
					<div>
						<h2>Device {done()}</h2>
						<p>You can close this tab and return to your device.</p>
						{/* TODO: maybe have a way to make the window/tab window.close() itself? if this is even possible or useful? */}
					</div>
				</Show>
				<Show when={!done()}>
					<ErrorBoundary
						fallback={(err) => <div class="error">error: {err.message}</div>}
					>
						<Show when={data.loading}>loading...</Show>
						<Show when={error()}>
							<div class="error">{error()}</div>
						</Show>
						<Show when={data()}>
							{(device) => (
								<div>
									<h2>
										Authorize <b>{device().application.name}</b>
									</h2>
									<Show when={device().application.description}>
										<div class="description">
											<h3 class="dim">app description</h3>
											<div style="height:8px" />
											<div class="contents">
												{device().application.description}
											</div>
										</div>
									</Show>
									<div class="info">
										This will grant <b>{device().application.name}</b> access
										to:
										<ul>
											<For each={device().scopes}>
												{(scope) => <li>{getScopeDescription(scope)}</li>}
											</For>
										</ul>
										<div style="height:8px" />
										<div class="dim">
											You are signed in as <b>{device().auth_user.name}</b>
										</div>
									</div>
									<menu>
										<button
											type="button"
											class="button big"
											onClick={() => decide(false)}
											disabled={busy()}
										>
											deny
										</button>
										<button
											type="button"
											class="button big primary"
											onClick={() => decide(true)}
											disabled={busy()}
										>
											{busy() ? "authorizing..." : "authorize"}
										</button>
									</menu>
								</div>
							)}
						</Show>
					</ErrorBoundary>
				</Show>
			</Show>
		</div>
	);
};
