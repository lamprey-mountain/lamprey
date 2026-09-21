import type { NavigateOptions, Navigator } from "@solidjs/router";
import {
	createContext,
	createSignal,
	type ParentProps,
	useContext,
} from "solid-js";

export type Router = {
	// TODO: implement and use this!
};

export const RouterContext = createContext<Router>();

export const RouterProvider = (props: ParentProps) => {
	// there are three separate locations
	// global location: displayed in the url bar and used in history
	// main layer location: room, channel, main content, etc
	// overlay layer location: settings, preferences, etc

	const [location, setLocation] = createSignal(window.location.pathname);
	const [main, setMain] = createSignal<string>();
	const [overlay, setOverlay] = createSignal<string>();

	// TODO: move history here
	// TODO: move useSettingsModals here
	// TODO: close settings modals on back, reopen on forward
	// TODO: location should be updated when either layer's location changes

	const router: Router = {
		// TODO
	};

	return (
		<RouterContext.Provider value={router}>
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
