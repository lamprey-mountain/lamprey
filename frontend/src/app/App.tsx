import type { RouteSectionProps } from "@solidjs/router";
import { Route, Router } from "@solidjs/router";
import {
	type Component,
	createEffect,
	For,
	from,
	type JSX,
	type ParentProps,
	Show,
} from "solid-js";
import { RootStoreContext } from "@/api";
import { chatctx, useCtx } from "@/app/context";
import { UserSettings } from "@/components/features/user_settings/index";
import { useVoice, VoiceProvider } from "@/components/features/voice/context";
import { VoiceStreams } from "@/components/features/voice/VoiceStreams";
import { CalendarPopupProvider } from "@/components/shared/Calendar";
import { Debug } from "@/components/shared/Debug";
import { RouteVerifyEmail } from "@/components/shared/VerifyEmail";
import {
	CurrentUserProvider,
	useCurrentUser,
} from "@/contexts/currentUser.tsx";
import { DisplayProvider } from "@/contexts/display.tsx";
import { MemberListProvider } from "@/contexts/memberlist.tsx";
import {
	AutocompleteProvider,
	FormattingToolbarProvider,
	MenuProvider,
	UserPopoutProvider,
} from "@/contexts/mod.tsx";
import { ModalsProvider, useModals } from "@/contexts/modal";
import { OverlayProvider } from "@/contexts/overlay.tsx";
import { ReadTrackingProvider } from "@/contexts/read-tracking.tsx";
import { SlashCommandsProvider } from "@/contexts/slash-commands.tsx";
import { UploadsProvider } from "@/contexts/uploads.tsx";
import { useAppConfig } from "@/hooks/useAppConfig.ts";
import { useChatClient } from "@/hooks/useChatClient.ts";
import { useFavicon } from "@/hooks/useFavicon.ts";
import { useGlobalEventHandlers } from "@/hooks/useGlobalEventHandlers.ts";
import { ConfigProvider, useConfig } from "@/lib/config";
import { flags } from "@/lib/flags";
import {
	AppLayoutMain,
	RouteAuthorize,
	RouteChannel,
	RouteChannelSettings,
	RouteFriends,
	RouteHome,
	RouteInbox,
	RouteInvite,
	RouteNotFound,
	RouteRoom,
	RouteRoomSettings,
	RouteSettings,
	RouteUser,
} from "@/routes";

const App: Component = () => {
	return (
		<Router root={AppBootstrap}>
			<Route path="/" component={AppLayoutMain}>
				<Route path="/" component={RouteHome} />
				<Route path="/inbox" component={RouteInbox} />
				<Route path="/friends" component={RouteFriends} />
				<Route path="/room/:room_id" component={RouteRoom} />
				<Route path="/channel/:channel_id" component={RouteChannel} />
				<Route
					path="/channel/:channel_id/message/:message_id"
					component={RouteChannel}
				/>
				<Route
					path="/channel/:channel_id/script/:script_id"
					component={RouteChannel}
				/>
				<Route path="/user/:user_id" component={RouteUser} />
				<Route path="/invite/:code" component={RouteInvite} />
				<Route path="/thread/:channel_id" component={RouteChannel} />
				<Route
					path="/thread/:channel_id/message/:message_id"
					component={RouteChannel}
				/>
			</Route>
			<Route path="/settings/:page?" component={RouteSettings} />
			<Route
				path="/room/:room_id/settings/:page?"
				component={RouteRoomSettings}
			/>
			<Route
				path="/channel/:channel_id/settings/:page?"
				component={RouteChannelSettings}
			/>
			<Route
				path="/thread/:channel_id/settings/:page?"
				component={RouteChannelSettings}
			/>
			<Route path="/debug" component={Debug} />
			<Route path="/verify-email" component={RouteVerifyEmail} />
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
export const AppBootstrap: Component<RouteSectionProps> = (props) => {
	const { config, resolved } = useAppConfig();

	return (
		<Show when={config()}>
			{(c) => (
				<ConfigProvider value={c()}>
					<AppProviders resolved={resolved()}>{props.children}</AppProviders>
				</ConfigProvider>
			)}
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
	const g = globalThis as typeof globalThis & {
		ctx: typeof ctx;
		client: typeof client;
		store: typeof store;
		flags: typeof flags;
	};
	g.ctx = ctx;
	g.client = client;
	g.store = store;
	g.flags = flags;

	return (
		<RootStoreContext.Provider value={store}>
			<CurrentUserProvider>
				<DisplayProvider>
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
				</DisplayProvider>
			</CurrentUserProvider>
		</RootStoreContext.Provider>
	);
};

/**
 * AppShell - Layer 3
 * Renders UI chrome, global event handlers, and overlay.
 */
export const AppShell: Component<ParentProps> = (props) => {
	const ctx = useCtx();
	const [modals] = useModals();

	useFavicon();
	useGlobalEventHandlers();

	const cursorStats = ctx.cursorStats;

	// HACK: set class/data-message-style for both root and overlay (for modals)
	return (
		<>
			<div
				id="root"
				class="root precedence-hack"
				classList={{
					"underline-links":
						ctx.preferences().frontend.underline_links === "yes",
				}}
				data-message-style={
					ctx.preferences().frontend.message_style === "compact"
						? "compact"
						: "cozy"
				}
				inert={modals.length > 0}
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
				<VoiceStreams />
			</div>
			<div
				id="overlay"
				class="root"
				classList={{
					"underline-links":
						ctx.preferences().frontend.underline_links === "yes",
				}}
				data-message-style={
					ctx.preferences().frontend.message_style === "compact"
						? "compact"
						: "cozy"
				}
			></div>
		</>
	);
};

export default App;
