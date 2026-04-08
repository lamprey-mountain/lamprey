// TODO: move to contexts/config.tsx

import { createContext, useContext } from "solid-js";

export type Config = {
	api_url: string;
	cdn_url: string;
};

const configCtx = createContext<Config | undefined>();

export const useConfig = (): Config => {
	const config = useContext(configCtx);
	if (!config) {
		throw new Error("useConfig must be used within a ConfigProvider");
	}
	return config;
};

export const ConfigProvider = configCtx.Provider;
