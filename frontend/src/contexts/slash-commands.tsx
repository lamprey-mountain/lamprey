import { createContext, type ParentProps, useContext } from "solid-js";
import type { SlashCommands } from "@/lib/commands/registry";

export const SlashCommandsContext = createContext<SlashCommands>();
export const useSlashCommands = () => useContext(SlashCommandsContext)!;

export function SlashCommandsProvider(
	props: ParentProps & { value: SlashCommands },
) {
	return (
		<SlashCommandsContext.Provider value={props.value}>
			{props.children}
		</SlashCommandsContext.Provider>
	);
}
