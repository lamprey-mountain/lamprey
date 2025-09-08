import { createContext, useContext } from "solid-js";

export type Config = {
	base_url: string;
	cdn_url: string;
};

const configCtx = createContext<Config>();

export const useConfig = () => {
	return useContext(configCtx)!;
};

export const ConfigProvider = configCtx.Provider;
