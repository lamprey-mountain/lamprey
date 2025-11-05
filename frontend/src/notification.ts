import { createSignal } from "solid-js";

type NotificationPermission = "granted" | "denied" | "prompt" | "unknown";

const [notificationPermission_, setNotificationPermission] = createSignal<
	NotificationPermission
>("unknown");
export const notificationPermission = notificationPermission_;

navigator.permissions.query({ name: "notifications" }).then((status) => {
	setNotificationPermission(status.state);
	status.addEventListener("change", () => {
		setNotificationPermission(status.state);
	});
});
