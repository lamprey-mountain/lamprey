import type { NavigateOptions, Navigator } from "@solidjs/router";
import {
	createContext,
	createEffect,
	createMemo,
	createSignal,
	on,
	onCleanup,
	type ParentProps,
	useContext,
} from "solid-js";
import { type Modal, useModals } from "./modal";

export type Router = {
	location: string;
	main: string;
	overlay: string;
};

export const RouterContext = createContext<Router>();

const getMainPath = (path: string) => {
	const userMatch = path.match(/^\/settings(\/([^/]+))?/);
	if (userMatch) {
		return "/";
	}

	const roomMatch = path.match(/^\/room\/([^/]+)\/settings(\/([^/]+))?/);
	if (roomMatch) {
		const [, room_id] = roomMatch;
		return `/room/${room_id}`;
	}

	const channelMatch = path.match(/^\/channel\/([^/]+)\/settings(\/([^/]+))?/);
	if (channelMatch) {
		const [, channel_id] = channelMatch;
		return `/channel/${channel_id}`;
	}

	return path;
};

export const useSettingsModals = () => {
	const router = useRouter();
	const [modals, modalCtl] = useModals();

	const isSettingsOpen = createMemo(() => {
		const lastModal = modals.at(-1);
		return (
			lastModal?.type === "user_settings" ||
			lastModal?.type === "room_settings" ||
			lastModal?.type === "channel_settings"
		);
	});

	const openSettings = (modal: Modal) => {
		if (isSettingsOpen()) {
			modalCtl.replace(modal);
		} else {
			modalCtl.open(modal);
		}
	};

	createEffect(
		on(
			() => router.overlay,
			(path) => {
				const userMatch = path.match(/^\/settings(\/([^/]+))?/);
				if (userMatch) {
					const [, , page] = userMatch;
					return openSettings({
						type: "user_settings",
						page,
					});
				}

				const roomMatch = path.match(/^\/room\/([^/]+)\/settings(\/([^/]+))?/);
				if (roomMatch) {
					const [, room_id, , page] = roomMatch;
					return openSettings({
						type: "room_settings",
						room_id,
						page,
					});
				}

				const channelMatch = path.match(
					/^\/channel\/([^/]+)\/settings(\/([^/]+))?/,
				);
				if (channelMatch) {
					const [, channel_id, , page] = channelMatch;
					return openSettings({
						type: "channel_settings",
						channel_id,
						page,
					});
				}

				if (isSettingsOpen()) {
					modalCtl.close();
				}
			},
		),
	);
};

export const createRouter = (): Router => {
	// there are three separate locations
	// global location: displayed in the url bar and used in history
	// main layer location: room, channel, main content, etc
	// overlay layer location: settings, preferences, etc

	const [location, setLocation] = createSignal(window.location.pathname);
	const [main, setMain] = createSignal(getMainPath(window.location.pathname));
	const [overlay, setOverlay] = createSignal("");

	const handlePopState = () => {
		const p = window.location.pathname;
		setLocation(p);

		// TODO: more robust parsing logic
		if (p.match(/\/settings/)) {
			setOverlay(p);
		} else {
			setMain(p);
			setOverlay("");
		}
	};

	handlePopState();

	const handleNavigate = () => {
		// TODO: handle the navigate event
		handlePopState();
	};

	window.navigation?.addEventListener("currententrychange", handleNavigate);
	window.addEventListener("popstate", handlePopState);
	onCleanup(() => {
		window.navigation?.removeEventListener(
			"currententrychange",
			handleNavigate,
		);
		window.removeEventListener("popstate", handlePopState);
	});

	return {
		get location() {
			return location();
		},
		get main() {
			return main();
		},
		get overlay() {
			return overlay();
		},
	};
};

export const RouterProvider = (props: ParentProps<{ router: Router }>) => {
	// TODO: move history here
	// TODO: close settings modals on back, reopen on forward
	// TODO: location should be updated when either layer's location changes

	return (
		<RouterContext.Provider value={props.router}>
			{props.children}
		</RouterContext.Provider>
	);
};

export const useRouter = () => {
	const ctx = useContext(RouterContext);
	if (!ctx) {
		throw new Error("useRouter must be used within a RouterProvider");
	}
	return ctx;
};

export const useNavigate = (): Navigator => {
	return (to: string | number, options?: Partial<NavigateOptions>) => {
		if (typeof to === "number") {
			window.history.go(to);
		} else {
			const replace = options?.replace ?? false;
			if (replace) {
				window.history.replaceState(null, "", to);
			} else {
				window.history.pushState(null, "", to);
			}
			window.dispatchEvent(new PopStateEvent("popstate"));
		}
	};
};
