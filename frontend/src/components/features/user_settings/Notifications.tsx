import { createEffect, Show, type VoidProps } from "solid-js";
import { type Preferences, type User } from "sdk";
import { Checkbox } from "../../../icons";
import { notificationPermission } from "../../../notification";
import { useCtx } from "../../../context";
import { Dropdown } from "../../../atoms/Dropdown";
import { useApi } from "@/api";
import { CheckboxOption } from "../../../atoms/CheckboxOption";

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
		field: keyof Preferences["notifs"],
		value:
			| NotifAction
			| NotifsMessages
			| NotifsThreads
			| NotifsReactions
			| NotifsTts,
	) => {
		const c = ctx.preferences();
		ctx.setPreferences({
			...c,
			notifs: {
				...c.notifs,
				[field]: value,
			},
		});
	};

	const setFrontendConfig = async (setting: string, value: string) => {
		const c = ctx.preferences();

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

					const vapidKey = serverInfo && typeof serverInfo === "object" &&
						"features" in serverInfo &&
						serverInfo.features && typeof serverInfo.features === "object" &&
						"web_push" in serverInfo.features &&
						serverInfo.features.web_push &&
						typeof serverInfo.features.web_push === "object" &&
						"vapid_public_key" in serverInfo.features.web_push &&
						serverInfo.features.web_push.vapid_public_key;

					if (!vapidKey) {
						console.error("No push info from backend");
						return;
					}

					const subscription = await registration.pushManager.subscribe({
						userVisibleOnly: true,
						applicationServerKey: urlBase64ToUint8Array(vapidKey),
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

		ctx.setPreferences({
			...c,
			frontend: {
				...c.frontend,
				[setting]: value,
			},
		});
	};

	const isFrontendConfigEnabled = (setting: string) => {
		return ctx.preferences().frontend[setting] === "yes";
	};

	createEffect(async () => {
		if ("serviceWorker" in navigator && "PushManager" in window) {
			const registration = await navigator.serviceWorker.ready;
			const subscription = await registration.pushManager.getSubscription();
			const isEnabled = !!subscription;
			const currentConfig = ctx.preferences().frontend["push_notifs"] === "yes";

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
			<CheckboxOption
				id={`user-${_props.user?.id ?? "@self"}-desktop-notifs`}
				checked={isFrontendConfigEnabled("desktop_notifs")}
				onChange={(checked) =>
					setFrontendConfig("desktop_notifs", checked ? "yes" : "no")}
				seed={`user-${_props.user?.id ?? "@self"}-desktop-notifs`}
			>
				<Checkbox
					checked={isFrontendConfigEnabled("desktop_notifs")}
					seed={`user-${_props.user?.id ?? "@self"}-desktop-notifs`}
				/>
				<div>
					<div>{t("user_settings.desktop_notifs")}</div>
					<div class="dim">{t("user_settings.desktop_notifs_description")}</div>
				</div>
			</CheckboxOption>
			<CheckboxOption
				id={`user-${_props.user?.id ?? "@self"}-push-notifs`}
				checked={isFrontendConfigEnabled("push_notifs")}
				onChange={(checked) =>
					setFrontendConfig("push_notifs", checked ? "yes" : "no")}
				seed={`user-${_props.user?.id ?? "@self"}-push-notifs`}
			>
				<Checkbox
					checked={isFrontendConfigEnabled("push_notifs")}
					seed={`user-${_props.user?.id ?? "@self"}-push-notifs`}
				/>
				<div>
					<div>{t("user_settings.push_notifs")}</div>
					<div class="dim">{t("user_settings.push_notifs_description")}</div>
				</div>
			</CheckboxOption>
			<CheckboxOption
				id={`user-${_props.user?.id ?? "@self"}-tts-notifs`}
				checked={isFrontendConfigEnabled("tts_notifs")}
				onChange={(checked) =>
					setFrontendConfig("tts_notifs", checked ? "yes" : "no")}
				seed={`user-${_props.user?.id ?? "@self"}-tts-notifs`}
			>
				<Checkbox
					checked={isFrontendConfigEnabled("tts_notifs")}
					seed={`user-${_props.user?.id ?? "@self"}-tts-notifs`}
				/>
				<div>
					<div>{t("user_settings.tts_notifs")}</div>
					<div class="dim">{t("user_settings.tts_notifs_description")}</div>
				</div>
			</CheckboxOption>
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
						selected={ctx.preferences().notifs.messages}
						onSelect={(value) =>
							value && setNotifConfig("messages", value as any)}
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
						selected={ctx.preferences().notifs.threads}
						onSelect={(value) =>
							value && setNotifConfig("threads", value as any)}
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
						selected={ctx.preferences().notifs.reactions}
						onSelect={(value) =>
							value && setNotifConfig("reactions", value as any)}
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
						selected={ctx.preferences().notifs.tts}
						onSelect={(value) => value && setNotifConfig("tts", value as any)}
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
