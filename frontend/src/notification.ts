import { createSignal } from "solid-js";
import type { Message } from "sdk";

type NotificationPermission = "granted" | "denied" | "prompt" | "unknown";

const [notificationPermission_, setNotificationPermission] = createSignal<
	NotificationPermission
>("unknown");
export const notificationPermission = notificationPermission_;

if (typeof navigator !== "undefined" && navigator.permissions) {
	navigator.permissions.query({ name: "notifications" } as any).then(
		(status) => {
			setNotificationPermission(status.state);
			status.addEventListener("change", () => {
				setNotificationPermission(status.state);
			});
		},
	);
}
