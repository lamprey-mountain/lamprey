import { Show, type VoidProps } from "solid-js";
import { type User, type UserConfig } from "sdk";
import { Checkbox } from "../icons";
import { notificationPermission } from "../notification";
import { useCtx } from "../context";
import { Dropdown } from "../Dropdown";

type NotifAction = "Notify" | "Watching" | "Ignore";

export function Notifications(_props: VoidProps<{ user: User }>) {
	const ctx = useCtx();
	const { t } = useCtx();

	// TODO: option to disable mention sound

	const setNotifConfig = (
		field: keyof UserConfig["notifs"],
		value: NotifAction,
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

	const setFrontendConfig = (setting: string, value: string) => {
		const c = ctx.userConfig();
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
							{ item: "Notify", label: t("user_settings.notify") },
							{ item: "Watching", label: t("user_settings.watching") },
							{ item: "Ignore", label: t("user_settings.ignore") },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>{t("user_settings.mentions")}</div>
						<div class="dim">
							{t("user_settings.mentions_description")}
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.mentions}
						onSelect={(value) => value && setNotifConfig("mentions", value)}
						options={[
							{ item: "Notify", label: t("user_settings.notify") },
							{ item: "Watching", label: t("user_settings.watching") },
							{ item: "Ignore", label: t("user_settings.ignore") },
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
							{ item: "Watching", label: t("user_settings.watching") },
							{ item: "Ignore", label: t("user_settings.ignore") },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>{t("user_settings.public_rooms")}</div>
						<div class="dim">
							{t("user_settings.public_rooms_description")}
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.room_public}
						onSelect={(value) => value && setNotifConfig("room_public", value)}
						options={[
							{ item: "Notify", label: t("user_settings.notify") },
							{ item: "Watching", label: t("user_settings.watching") },
							{ item: "Ignore", label: t("user_settings.ignore") },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>{t("user_settings.private_rooms")}</div>
						<div class="dim">
							{t("user_settings.private_rooms_description")}
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.room_private}
						onSelect={(value) => value && setNotifConfig("room_private", value)}
						options={[
							{ item: "Notify", label: t("user_settings.notify") },
							{ item: "Watching", label: t("user_settings.watching") },
							{ item: "Ignore", label: t("user_settings.ignore") },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>{t("user_settings.direct_messages")}</div>
						<div class="dim">
							{t("user_settings.direct_messages_description")}
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.room_dm}
						onSelect={(value) => value && setNotifConfig("room_dm", value)}
						options={[
							{ item: "Notify", label: t("user_settings.notify") },
							{ item: "Watching", label: t("user_settings.watching") },
							{ item: "Ignore", label: t("user_settings.ignore") },
						]}
					/>
				</div>
			</div>
		</div>
	);
}
