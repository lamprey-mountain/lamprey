import type { ChatCtx } from "@/types/chat";
import type { Command, OptionDef } from "./types";
import type { ChannelsService, RootStore } from "@/api";

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

	parse(
		text: string,
	): { command: Command; args: Record<string, unknown> } | undefined {
		const parts = text.slice(1).split(" ");
		const top = this.find(parts[0]);
		if (!top) return undefined;

		let command = top;
		let rawArgs = parts.slice(1);

		// descend through arbitrarily many levels, e.g. `/room settings notifications on`
		while (command.subcommands?.length) {
			const sub = command.subcommands.find((s) => s.name === rawArgs[0]);
			if (!sub) break;
			command = sub;
			rawArgs = rawArgs.slice(1);
		}

		// TODO: parse options from parent command?
		return { command, args: parseOptions(command.options, rawArgs) };
	}

	// TODO: clean up type signature
	async run(
		_ctx: ChatCtx,
		api: RootStore,
		_channels: ChannelsService,
		channel_id: string,
		text: string,
		_store: RootStore,
	) {
		const parsed = this.parse(text);
		if (!parsed) {
			// TODO: better error reporting
			const commandName = text.slice(1).split(" ")[0];
			console.error(`Command not found: ${commandName}`);
			return;
		}

		const channel = api.channels.cache.get(channel_id);
		if (!channel) {
			console.error(`Channel not found: ${channel_id}`);
			return;
		}

		if (!parsed.command.canUse(api, channel)) {
			console.error(`Insufficient permissions to use /${parsed.command.name}`);
			return;
		}

		parsed.command.execute(
			api,
			channel,
			parsed.args as any, // TODO: better typing
		);
	}
}

export function parseOptions(
	options: OptionDef[],
	rawArgs: string[],
): Record<string, unknown> {
	const args: Record<string, unknown> = {};
	let idx = 0;
	options.forEach((opt, i) => {
		const isLast = i === options.length - 1;
		if (opt.kind === "string" && isLast) {
			args[opt.name] = rawArgs.slice(idx).join(" ") || undefined;
			idx = rawArgs.length;
		} else if (opt.kind === "duration") {
			args[opt.name] =
				rawArgs[idx] !== undefined ? parseInt(rawArgs[idx], 10) : undefined;
			idx++;
		} else {
			args[opt.name] = rawArgs[idx];
			idx++;
		}
	});
	return args;
}
