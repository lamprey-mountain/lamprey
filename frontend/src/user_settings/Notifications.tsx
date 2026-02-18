import { createEffect, Show, type VoidProps } from "solid-js";
import { type User, type UserConfig } from "sdk";
import { Checkbox } from "../icons";
import { notificationPermission } from "../notification";
import { useCtx } from "../context";
import { Dropdown } from "../Dropdown";
import { useApi } from "../api";

type NotifAction = "Notify" | "Watching" | "Ignore";
type NotifsMessages = "Everything" | "Watching" | "Mentions" | "Nothing";
type NotifsThreads = "Notify" | "Inbox" | "Nothing";
type NotifsReactions = "Always" | "Restricted" | "Dms" | "Nothing";
type NotifsTts = "Always" | "Mentions" | "Nothing";

function urlBase64ToUint8Array(base64String: string) {
	const padding = "=".repeat((4 - (base64String.length % 4)) % 4);
	const base64 = (base64String + padding).replace(/-/g, "+").replace(/_/g, "/");
	const rawData = window.atob(base64);
	const outputArray = new Uint8Array(rawData.length);
	for (let i = 0; i < rawData.length; ++i) {
		outputArray[i] = rawData.charCodeAt(i);
	}
	return outputArray;
}

export function Notifications(_props: VoidProps<{ user: User }>) {
	const ctx = useCtx();
	const api = useApi();
	const { t } = useCtx();

	// TODO: option to disable mention sound

	const setNotifConfig = (
		field: keyof UserConfig["notifs"],
		value:
			| NotifAction
			| NotifsMessages
			| NotifsThreads
			| NotifsReactions
			| NotifsTts,
	) => {
		const c = ctx.userConfig();
		ctx.setUserConfig({
			...c,
			notifs: {
				...c.notifs,
				[field]: value,
			},
		});
	};

	const setFrontendConfig = async (setting: string, value: string) => {
		const c = ctx.userConfig();

		if (setting === "push_notifs") {
			if (value === "yes") {
				try {
					const permission = await Notification.requestPermission();
					if (permission !== "granted") {
						throw new Error("Permission not granted");
					}

					const registration = await navigator.serviceWorker.ready;
					const serverInfo = await api.client.http.GET("/api/v1/server/@self")
						.then((res) => res.data);

					if (!serverInfo?.features.web_push?.vapid_public_key) {
						console.error("No push info from backend");
						return;
					}

					const subscription = await registration.pushManager.subscribe({
						userVisibleOnly: true,
						applicationServerKey: urlBase64ToUint8Array(
							serverInfo.features.web_push.vapid_public_key,
						),
					});

					const subJson = subscription.toJSON();
					await api.push.register({
						endpoint: subJson.endpoint!,
						keys: {
							p256dh: subJson.keys!.p256dh!,
							auth: subJson.keys!.auth!,
						},
					});
				} catch (e) {
					console.error("Failed to subscribe to push notifications", e);
					return;
				}
			} else {
				try {
					const registration = await navigator.serviceWorker.ready;
					const subscription = await registration.pushManager.getSubscription();
					if (subscription) {
						await subscription.unsubscribe();
					}
					await api.push.delete();
				} catch (e) {
					console.error("Failed to unsubscribe from push notifications", e);
				}
			}
		}

		ctx.setUserConfig({
			...c,
			frontend: {
				...c.frontend,
				[setting]: value,
			},
		});
	};

	const isFrontendConfigEnabled = (setting: string) => {
		return ctx.userConfig().frontend[setting] === "yes";
	};

	createEffect(async () => {
		if ("serviceWorker" in navigator && "PushManager" in window) {
			const registration = await navigator.serviceWorker.ready;
			const subscription = await registration.pushManager.getSubscription();
			const isEnabled = !!subscription;
			const currentConfig = ctx.userConfig().frontend["push_notifs"] === "yes";

			if (isEnabled !== currentConfig) {
				// TODO: sync config
			}
		}
	});

	return (
		<div class="user-settings-notifications">
			<h2>{t("user_settings.notifications")}</h2>
			<Show when={notificationPermission() !== "granted"}>
				<div class="permission">
					{t("user_settings.notifications_permission_text")}
					<button
						class="primary"
						onClick={() => Notification.requestPermission()}
					>
						{t("user_settings.notifications_permission_button")}
					</button>
				</div>
			</Show>
			<label class="option">
				<input
					type="checkbox"
					checked={isFrontendConfigEnabled("desktop_notifs")}
					onInput={(e) =>
						setFrontendConfig(
							"desktop_notifs",
							e.target.checked ? "yes" : "no",
						)}
					style="display: none;"
				/>
				<Checkbox checked={isFrontendConfigEnabled("desktop_notifs")} />
				<div>
					<div>{t("user_settings.desktop_notifs")}</div>
					<div class="dim">{t("user_settings.desktop_notifs_description")}</div>
				</div>
			</label>
			<label class="option">
				<input
					type="checkbox"
					checked={isFrontendConfigEnabled("push_notifs")}
					onInput={(e) =>
						setFrontendConfig("push_notifs", e.target.checked ? "yes" : "no")}
					style="display: none;"
				/>
				<Checkbox checked={isFrontendConfigEnabled("push_notifs")} />
				<div>
					<div>{t("user_settings.push_notifs")}</div>
					<div class="dim">{t("user_settings.push_notifs_description")}</div>
				</div>
			</label>
			<label class="option">
				<input
					type="checkbox"
					checked={isFrontendConfigEnabled("tts_notifs")}
					onInput={(e) =>
						setFrontendConfig("tts_notifs", e.target.checked ? "yes" : "no")}
					style="display: none;"
				/>
				<Checkbox checked={isFrontendConfigEnabled("tts_notifs")} />
				<div>
					<div>{t("user_settings.tts_notifs")}</div>
					<div class="dim">{t("user_settings.tts_notifs_description")}</div>
				</div>
			</label>
			<h3 class="dim" style="margin-top:8px;margin-left:4px">
				{t("user_settings.notifications_more_stuff")}
			</h3>
			<div class="options dropdowns">
				<div class="option">
					<div>
						<div>{t("user_settings.messages")}</div>
						<div class="dim">
							{t("user_settings.messages_description")}
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.messages}
						onSelect={(value) => value && setNotifConfig("messages", value)}
						options={[
							{ item: "Everything", label: t("user_settings.everything") },
							{ item: "Watching", label: t("user_settings.watching") },
							{ item: "Mentions", label: t("user_settings.mentions_only") },
							{ item: "Nothing", label: t("user_settings.nothing") },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>{t("user_settings.threads")}</div>
						<div class="dim">
							{t("user_settings.threads_description")}
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.threads}
						onSelect={(value) => value && setNotifConfig("threads", value)}
						options={[
							{ item: "Notify", label: t("user_settings.notify") },
							{ item: "Inbox", label: t("user_settings.inbox") },
							{ item: "Nothing", label: t("user_settings.nothing") },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>{t("user_settings.reactions")}</div>
						<div class="dim">
							{t("user_settings.reactions_description")}
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.reactions}
						onSelect={(value) => value && setNotifConfig("reactions", value)}
						options={[
							{ item: "Always", label: t("user_settings.always") },
							{ item: "Restricted", label: t("user_settings.restricted") },
							{ item: "Dms", label: t("user_settings.direct_messages_only") },
							{ item: "Nothing", label: t("user_settings.nothing") },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>{t("user_settings.tts")}</div>
						<div class="dim">
							{t("user_settings.tts_description")}
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.tts}
						onSelect={(value) => value && setNotifConfig("tts", value)}
						options={[
							{ item: "Always", label: t("user_settings.always") },
							{ item: "Mentions", label: t("user_settings.mentions_only") },
							{ item: "Nothing", label: t("user_settings.nothing") },
						]}
					/>
				</div>
			</div>
		</div>
	);
}
