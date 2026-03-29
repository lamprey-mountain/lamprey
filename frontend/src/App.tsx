import {
	type Component,
	createEffect,
	For,
	from,
	type ParentProps,
	Show,
} from "solid-js";
import type { RouteSectionProps } from "@solidjs/router";
import { Portal } from "solid-js/web";
import { Route, Router } from "@solidjs/router";
import { Debug } from "./Debug.tsx";
import { UserSettings } from "./UserSettings.tsx";
import { chatctx, useCtx } from "./context.ts";
import { Config, ConfigProvider, useConfig } from "./config.tsx";
import { useAppConfig } from "./hooks/useAppConfig.ts";
import { useChatClient } from "./hooks/useChatClient.ts";
import { useFavicon } from "./hooks/useFavicon.ts";
import { useGlobalEventHandlers } from "./hooks/useGlobalEventHandlers.ts";
import { OverlayProvider } from "./contexts/overlay.tsx";
import {
	useVoice,
	VoiceProvider,
} from "./components/features/voice/voice-provider.tsx";
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
} from "./routes";
import { RouteVerifyEmail } from "./VerifyEmail.tsx";
import { CalendarPopupProvider } from "./Calendar.tsx";
import { ModalsProvider, useModals } from "./contexts/modal";
import { MemberListProvider } from "./contexts/memberlist.tsx";
import { UploadsProvider } from "./contexts/uploads.tsx";
import { SlashCommandsProvider } from "./contexts/slash-commands.tsx";
import { ReadTrackingProvider } from "./contexts/read-tracking.tsx";
import {
	AutocompleteProvider,
	FormattingToolbarProvider,
	MenuProvider,
	UserPopoutProvider,
} from "./contexts/mod.tsx";
import { RootStoreContext, useApi } from "@/api";
import {
	CurrentUserProvider,
	useCurrentUser,
} from "./contexts/currentUser.tsx";
import { flags } from "./flags.ts";

const App: Component = () => {
	return (
		<Router root={AppBootstrap}>
			<Route path="/" component={RouteHome as any} />
			<Route path="/inbox" component={RouteInbox as any} />
			<Route path="/friends" component={RouteFriends as any} />
			<Route path="/settings/:page?" component={RouteSettings as any} />
			<Route path="/room/:room_id" component={RouteRoom as any} />
			<Route
				path="/room/:room_id/settings/:page?"
				component={RouteRoomSettings as any}
			/>
			<Route
				path="/channel/:channel_id/settings/:page?"
				component={RouteChannelSettings as any}
			/>
			<Route path="/channel/:channel_id" component={RouteChannel as any} />
			<Route
				path="/channel/:channel_id/message/:message_id"
				component={RouteChannel as any}
			/>
			<Route
				path="/thread/:channel_id/settings/:page?"
				component={RouteChannelSettings as any}
			/>
			<Route path="/thread/:channel_id" component={RouteChannel as any} />
			<Route
				path="/thread/:channel_id/message/:message_id"
				component={RouteChannel as any}
			/>
			<Route path="/debug" component={Debug as any} />
			<Route path="/feed" component={RouteFeed as any} />
			<Route path="/invite/:code" component={RouteInvite as any} />
			<Route path="/verify-email" component={RouteVerifyEmail as any} />
			<Route path="/user/:user_id" component={RouteUser as any} />
			<Route path="/authorize" component={RouteAuthorize as any} />
			<Route path="*404" component={RouteNotFound} />
		</Router>
	);
};

/**
 * AppBootstrap - Layer 1
 * Handles config loading and provides ConfigProvider.
 * Renders conditionally until config is available.
 */
export const AppBootstrap: Component<RouteSectionProps> = (props) => {
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
export const AppProviders: Component<ParentProps<{ resolved: boolean }>> = (
	props,
) => {
	const config = useConfig();
	const { client, ctx, store } = useChatClient(config);

	// TEMP: debugging
	(globalThis as any).ctx = ctx;
	(globalThis as any).client = client;
	(globalThis as any).store = store;
	(globalThis as any).flags = flags;

	return (
		<RootStoreContext.Provider value={store}>
			<CurrentUserProvider>
				<chatctx.Provider value={ctx}>
					<ReadTrackingProvider
						api={store}
						channels2={store.channels}
						channel_contexts={ctx.channel_contexts}
						dataUpdate={ctx.dataUpdate}
					>
						<MemberListProvider>
							<ModalsProvider>
								<UploadsProvider ctx={ctx}>
									<VoiceProvider>
										<SlashCommandsProvider value={ctx.slashCommands}>
											<MenuProvider>
												<AutocompleteProvider>
													<FormattingToolbarProvider>
														<UserPopoutProvider>
															<CalendarPopupProvider>
																<AppShell>{props.children}</AppShell>
															</CalendarPopupProvider>
														</UserPopoutProvider>
													</FormattingToolbarProvider>
												</AutocompleteProvider>
											</MenuProvider>
										</SlashCommandsProvider>
									</VoiceProvider>
								</UploadsProvider>
							</ModalsProvider>
						</MemberListProvider>
					</ReadTrackingProvider>
				</chatctx.Provider>
			</CurrentUserProvider>
		</RootStoreContext.Provider>
	);
};

/**
 * AppShell - Layer 3
 * Renders UI chrome, global event handlers, and overlay.
 */
export const AppShell: Component<ParentProps<{}>> = (props) => {
	const ctx = useCtx();
	const [voice] = useVoice();
	const state = from(ctx.client.state);

	useFavicon();
	useGlobalEventHandlers();

	const cursorStats = ctx.cursorStats;

	return (
		<div
			id="root"
			class="precedence-hack"
			classList={{
				"underline-links":
					ctx.preferences().frontend["underline_links"] === "yes",
			}}
		>
			<Show when={cursorStats()}>
				{(stats) => (
					<div
						class="cursor-tooltip"
						style={{
							position: "fixed",
							top: `${stats().y + 16}px`,
							left: `${stats().x + 16}px`,
							"z-index": 10000,
							background: "oklch(var(--color-bg2) / 0.9)",
							color: "oklch(var(--color-fg1))",
							border: "1px solid oklch(var(--color-sep-300))",
							padding: "4px 8px",
							"border-radius": "4px",
							"pointer-events": "none",
							"font-size": "12px",
							"white-space": "nowrap",
							"backdrop-filter": "blur(4px)",
						}}
					>
						{stats().label}
					</div>
				)}
			</Show>
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
							const c = voice.preferences.get(stream.user_id) ??
								{ mute: false, mute_video: false, volume: 100 };
							audioRef.volume = c.volume / 100;
						});
						return (
							<audio
								autoplay
								ref={audioRef!}
								muted={voice.deafened ||
									voice.preferences.get(stream.user_id)?.mute === true}
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
	const user = useCurrentUser();
	createEffect(() => {
		console.log(user());
	});
	return (
		<>
			<Title title={user() ? t("page.settings_user") : t("loading")} />
			<Show when={user()}>
				<UserSettings user={user()!} page={p.params.page!} />
			</Show>
		</>
	);
}

export default App;
