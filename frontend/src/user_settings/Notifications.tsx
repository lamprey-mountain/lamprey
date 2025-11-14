import { Show, type VoidProps } from "solid-js";
import { type User, type UserConfig } from "sdk";
import { Checkbox } from "../icons";
import { notificationPermission } from "../notification";
import { useCtx } from "../context";
import { Dropdown } from "../Dropdown";

type NotifAction = "Notify" | "Watching" | "Ignore";

export function Notifications(_props: VoidProps<{ user: User }>) {
	const ctx = useCtx();

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
			<h2>notifications</h2>
			<Show when={notificationPermission() !== "granted"}>
				<div class="permission">
					You haven't given lamprey permission to send notifications
					<button
						class="primary"
						onClick={() => Notification.requestPermission()}
					>
						Allow notifications
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
					<div>Enable desktop notifications</div>
					<div class="dim">Show desktop notifications for messages</div>
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
					<div>Enable push notifications</div>
					<div class="dim">Receive push notifications when away</div>
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
					<div>Enable text to speech for notifications</div>
					<div class="dim">Read notification messages aloud</div>
				</div>
			</label>
			<h3 class="dim" style="margin-top:8px;margin-left:4px">more stuff</h3>
			<div class="options dropdowns">
				<div class="option">
					<div>
						<div>Messages</div>
						<div class="dim">
							Configure how you want to be notified of new messages
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.messages}
						onSelect={(value) => value && setNotifConfig("messages", value)}
						options={[
							{ item: "Notify", label: "Notify" },
							{ item: "Watching", label: "Watching" },
							{ item: "Ignore", label: "Ignore" },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>Mentions</div>
						<div class="dim">
							Configure how you want to be notified of mentions
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.mentions}
						onSelect={(value) => value && setNotifConfig("mentions", value)}
						options={[
							{ item: "Notify", label: "Notify" },
							{ item: "Watching", label: "Watching" },
							{ item: "Ignore", label: "Ignore" },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>Threads</div>
						<div class="dim">
							Configure how you want to be notified of new threads
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.threads}
						onSelect={(value) => value && setNotifConfig("threads", value)}
						options={[
							{ item: "Notify", label: "Notify" },
							{ item: "Watching", label: "Watching" },
							{ item: "Ignore", label: "Ignore" },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>Public Rooms</div>
						<div class="dim">
							Configure how you want to be notified of public room activity
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.room_public}
						onSelect={(value) => value && setNotifConfig("room_public", value)}
						options={[
							{ item: "Notify", label: "Notify" },
							{ item: "Watching", label: "Watching" },
							{ item: "Ignore", label: "Ignore" },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>Private Rooms</div>
						<div class="dim">
							Configure how you want to be notified of private room activity
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.room_private}
						onSelect={(value) => value && setNotifConfig("room_private", value)}
						options={[
							{ item: "Notify", label: "Notify" },
							{ item: "Watching", label: "Watching" },
							{ item: "Ignore", label: "Ignore" },
						]}
					/>
				</div>
				<div class="option">
					<div>
						<div>Direct Messages</div>
						<div class="dim">
							Configure how you want to be notified of direct messages
						</div>
					</div>
					<Dropdown
						selected={ctx.userConfig().notifs.room_dm}
						onSelect={(value) => value && setNotifConfig("room_dm", value)}
						options={[
							{ item: "Notify", label: "Notify" },
							{ item: "Watching", label: "Watching" },
							{ item: "Ignore", label: "Ignore" },
						]}
					/>
				</div>
			</div>
		</div>
	);
}
