import { Channel } from "sdk";
import type { Api } from "./api.tsx";
import type { ChatCtx } from "./context.ts";
import { createContext, useContext } from "solid-js";

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
	canUse?: (api: Api, room_id: string | undefined, channel: Channel) => boolean;
	execute: (
		ctx: ChatCtx,
		api: Api,
		channel_id: string,
		args: string[],
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

	async run(ctx: ChatCtx, api: Api, channel_id: string, text: string) {
		const [cmd, ...args] = text.slice(1).split(" ");
		const command = this.find(cmd);
		if (!command) {
			console.error(`Command not found: ${cmd}`);
			return;
		}
		await command.execute(ctx, api, channel_id, args);
	}
}

export const SlashCommandsContext = createContext<SlashCommands>();
export const useSlashCommands = () => useContext(SlashCommandsContext)!;
