import { createContext, type ParentProps, useContext } from "solid-js";

const MOBILE_BREAKPOINT = 768;

type DisplayContextType = {
	isMobile: () => boolean;
};

export const DisplayContext = createContext<DisplayContextType>();

export const DisplayProvider = (props: ParentProps) => {
	return (
		<DisplayContext.Provider
			value={{ isMobile: () => window.innerWidth < MOBILE_BREAKPOINT }}
		>
			{props.children}
		</DisplayContext.Provider>
	);
};

export const useDisplay = () => {
	const ctx = useContext(DisplayContext);
	if (!ctx) {
		throw new Error("useDisplay must be used within DisplayProvider");
	}
	return ctx;
};
