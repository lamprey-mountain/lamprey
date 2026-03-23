import { Channel } from "sdk";
import type { Api } from "@/api";
import type { ChannelsService } from "@/api/services/ChannelsService";
import type { ChatCtx } from "../context.ts";
import { createContext, type ParentProps, useContext } from "solid-js";
import { RootStore } from "@/api/core/Store.ts";

export type CommandOption = {
	name: string;
	description: string;
	type: "string" | "user" | "duration";
	required?: boolean;
};

export type Command = {
	id: string;
	name: string;
	description: string;
	options: CommandOption[];
	canUse?: (
		api: Api,
		channels: ChannelsService,
		room_id: string | undefined,
		channel: Channel,
		store: RootStore,
	) => boolean;
	execute: (
		ctx: ChatCtx,
		api: Api,
		channels: ChannelsService,
		channel_id: string,
		args: string[],
		store: RootStore,
	) => Promise<void>;
};

export class SlashCommands {
	private commands: Command[] = [];

	register(command: Command) {
		this.commands.push(command);
	}

	getAll(): Command[] {
		return [...this.commands];
	}

	find(name: string): Command | undefined {
		return this.commands.find((cmd) => cmd.name === name);
	}

	async run(
		ctx: ChatCtx,
		api: Api,
		channels: ChannelsService,
		channel_id: string,
		text: string,
		store: RootStore,
	) {
		const [cmd, ...args] = text.slice(1).split(" ");
		const command = this.find(cmd);
		if (!command) {
			console.error(`Command not found: ${cmd}`);
			return;
		}
		await command.execute(ctx, api, channels, channel_id, args, store);
	}
}

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
