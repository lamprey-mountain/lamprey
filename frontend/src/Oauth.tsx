import { Location, RouteSectionProps, useNavigate } from "@solidjs/router";
import { useCtx } from "./context";
import { createResource, ErrorBoundary, For, Show, VoidProps } from "solid-js";
import { OauthInfo } from "sdk";
import { Avatar } from "./User";

export const RouteAuthorize = (p: RouteSectionProps) => {
	const ctx = useCtx();

	const [data] = createResource(async () => {
		// HACK: openapi fetch typescript doesn't like manually built urls here
		const { data, error } = await ctx.client.http.GET(
			("/api/v1/oauth/authorize" + p.location.search) as any,
			{},
		);
		if (error) throw error.error;
		return data;
	});

	return (
		<div class="oauth-authorize">
			<ErrorBoundary
				fallback={() => (
					<div class="error">error: {(data.error as Error)?.message}</div>
				)}
			>
				<Show when={data.loading}>
					loading...
				</Show>
				<Show when={data()}>
					<OauthAuthorizePrompt
						application={data().application}
						auth_user={data().auth_user}
						bot_user={data().bot_user}
						authorized={data().authorized}
						location={p.location}
					/>
				</Show>
			</ErrorBoundary>
		</div>
	);
};

export const OauthAuthorizePrompt = (
	p: VoidProps<OauthInfo & { location: Location }>,
) => {
	const ctx = useCtx();
	const nav = useNavigate();

	const authorize = async () => {
		try {
			const { data, error } = await ctx.client.http.POST(
				("/api/v1/oauth/authorize" + p.location.search) as any,
				{},
			);
			if (error) {
				console.error(error);
			} else {
				location.href = data.redirect_uri;
			}
		} catch (err) {
			console.error(err);
		}
	};

	const scopes = () => {
		const s = p.location.query.scope;
		const s2 = Array.isArray(s) ? s : [s];
		return s2.flatMap((i) => i.split(" "));
	};

	const cancel = () => {
		window.close();
		nav("/");
	};

	return (
		<div>
			<h2>
				Authorize <b>{p.application.name}</b>
			</h2>

			<Show when={p.application.description}>
				<div class="description">
					<h3 class="dim">app description</h3>
					<div style="height:8px" />
					<div class="contents">{p.application.description}</div>
				</div>
			</Show>

			<div class="info">
				This will grant <b>{p.application.name}</b> access to:
				<ul>
					<For each={scopes()}>{(i) => <li>{getScopeInfo(i)}</li>}</For>
				</ul>
				<div style="height:8px" />
				<div class="dim">
					You are signed in as <b>{p.auth_user.name}</b>
				</div>
			</div>
			<menu>
				<button class="big" onClick={cancel}>cancel</button>
				<button class="big primary" onClick={authorize}>authorize</button>
			</menu>
		</div>
	);
};

function getScopeInfo(s: string) {
	switch (s) {
		case "openid":
		case "identify":
			return "basic profile information";
		case "full":
			return "full access to your account";
		case "auth":
			return "full access, including authorization information";
		default:
			return `!!! UNKNOWN OR INVALID SCOPE "${s}" !!!`;
	}
}
