import { type Component, createEffect, For, from, Show } from "solid-js";
import { type ParentProps as RouterParentProps } from "@solidjs/router";
import { Portal } from "solid-js/web";
import { Route, Router, type RouteSectionProps } from "@solidjs/router";
import { Debug } from "./Debug.tsx";
import { UserSettings } from "./UserSettings.tsx";
import { chatctx, useCtx } from "./context.ts";
import { Config, ConfigProvider, useConfig } from "./config.tsx";
import { useAppConfig } from "./hooks/useAppConfig.ts";
import { useChatClient } from "./hooks/useChatClient.ts";
import { useFavicon } from "./hooks/useFavicon.ts";
import { useGlobalEventHandlers } from "./hooks/useGlobalEventHandlers.ts";
import { OverlayProvider } from "./contexts/overlay.tsx";
import { useVoice, VoiceProvider } from "./voice-provider.tsx";
import {
	RouteAuthorize,
	RouteChannel,
	RouteChannelSettings,
	RouteFeed,
	RouteFriends,
	RouteHome,
	RouteInbox,
	RouteInvite,
	RouteNotFound,
	RouteRoom,
	RouteRoomSettings,
	RouteUser,
} from "./routes.tsx";
import { RouteVerifyEmail } from "./VerifyEmail.tsx";
import { ModalsProvider, useModals } from "./contexts/modal";
import { MemberListProvider } from "./contexts/memberlist.tsx";
import { UploadsProvider } from "./contexts/uploads.tsx";
import { SlashCommandsContext } from "./slash-commands.ts";
import { useApi } from "./api.tsx";

const App: Component = () => {
	return (
		<Router root={AppBootstrap}>
			<Route path="/" component={RouteHome} />
			<Route path="/inbox" component={RouteInbox} />
			<Route path="/friends" component={RouteFriends} />
			<Route path="/settings/:page?" component={RouteSettings} />
			<Route path="/room/:room_id" component={RouteRoom} />
			<Route
				path="/room/:room_id/settings/:page?"
				component={RouteRoomSettings}
			/>
			<Route
				path="/channel/:channel_id/settings/:page?"
				component={RouteChannelSettings}
			/>
			<Route path="/channel/:channel_id" component={RouteChannel} />
			<Route
				path="/channel/:channel_id/message/:message_id"
				component={RouteChannel}
			/>
			<Route
				path="/thread/:channel_id/settings/:page?"
				component={RouteChannelSettings}
			/>
			<Route path="/thread/:channel_id" component={RouteChannel} />
			<Route
				path="/thread/:channel_id/message/:message_id"
				component={RouteChannel}
			/>
			<Route path="/debug" component={Debug} />
			<Route path="/feed" component={RouteFeed} />
			<Route path="/invite/:code" component={RouteInvite} />
			<Route path="/verify-email" component={RouteVerifyEmail} />
			<Route path="/user/:user_id" component={RouteUser} />
			<Route path="/authorize" component={RouteAuthorize} />
			<Route path="*404" component={RouteNotFound} />
		</Router>
	);
};

/**
 * AppBootstrap - Layer 1
 * Handles config loading and provides ConfigProvider.
 * Renders conditionally until config is available.
 */
export const AppBootstrap: Component<RouterParentProps> = (props) => {
	const { config, resolved } = useAppConfig();

	return (
		<Show when={config()}>
			<ConfigProvider value={config()!}>
				<AppProviders resolved={resolved()}>{props.children}</AppProviders>
			</ConfigProvider>
		</Show>
	);
};

/**
 * AppProviders - Layer 2
 * Initializes client, API, and all context providers.
 */
export const AppProviders: Component<
	RouterParentProps & { resolved: boolean }
> = (props) => {
	const config = useConfig();
	const { client, api, ctx } = useChatClient(config);

	return (
		<api.Provider>
			<chatctx.Provider value={ctx}>
				<MemberListProvider>
					<ModalsProvider>
						<UploadsProvider ctx={ctx}>
							<VoiceProvider>
								<SlashCommandsContext.Provider value={ctx.slashCommands}>
									<AppShell>{props.children}</AppShell>
								</SlashCommandsContext.Provider>
							</VoiceProvider>
						</UploadsProvider>
					</ModalsProvider>
				</MemberListProvider>
			</chatctx.Provider>
		</api.Provider>
	);
};

/**
 * AppShell - Layer 3
 * Renders UI chrome, global event handlers, and overlay.
 */
export const AppShell: Component<RouterParentProps> = (props) => {
	const ctx = useCtx();
	const [voice] = useVoice();
	const state = from(ctx.client.state);

	useFavicon();
	useGlobalEventHandlers({ setMenu: ctx.setMenu });

	return (
		<div
			id="root"
			classList={{
				"underline-links":
					ctx.userConfig().frontend["underline_links"] === "yes",
			}}
		>
			{props.children}
			<OverlayProvider />
			<div style="visibility:hidden">
				<For each={[...voice.rtc?.streams.values() ?? []]}>
					{(stream) => {
						let audioRef!: HTMLAudioElement;
						createEffect(() => {
							console.log("listening to stream", stream);
							if (audioRef) audioRef.srcObject = stream.media;
						});
						createEffect(() => {
							const c = voice.userConfig.get(stream.user_id) ??
								{ mute: false, mute_video: false, volume: 100 };
							audioRef.volume = c.volume / 100;
						});
						return (
							<audio
								autoplay
								ref={audioRef!}
								muted={voice.deafened ||
									voice.userConfig.get(stream.user_id)?.mute === true}
							/>
						);
					}}
				</For>
			</div>
			<Show when={state() !== "ready"}>
				<div style="position:fixed;top:8px;left:8px;background:#111;padding:8px;border:solid #222 1px;">
					{state()}
				</div>
			</Show>
		</div>
	);
};

const Title = (props: { title?: string }) => {
	createEffect(() => document.title = props.title ?? "");
	return undefined;
};

function RouteSettings(p: RouteSectionProps) {
	const { t } = useCtx();
	const api = useApi();
	const user = () => api.users.cache.get("@self");
	createEffect(() => {
		console.log(user());
	});
	return (
		<>
			<Title title={user() ? t("page.settings_user") : t("loading")} />
			<Show when={user()}>
				<UserSettings user={user()!} page={p.params.page} />
			</Show>
		</>
	);
}

export default App;
