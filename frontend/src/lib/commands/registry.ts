import type { Channel } from "sdk";
import type { ChannelsService, RootStore } from "@/api";
import type { ChatCtx } from "@/types/chat";
import { parseDuration } from "./duration";
import { tokenizeArgs } from "./tokenize";
import type { Command, OptionDef } from "./types";

export type CommandMatch = { command: Command; consumed: number };

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

	/** resolve `/command subcommand ...`, returning the deepest command and the
	 * length of the matched prefix (including the leading slash) */
	match(text: string): CommandMatch | undefined {
		if (!text.startsWith("/")) return undefined;
		const parts = text.slice(1).split(" ");
		const top = this.find(parts[0] ?? "");
		if (!top) return undefined;

		let command = top;
		let consumed = 1 + (parts[0]?.length ?? 0);
		let idx = 1;

		while (command.subcommands?.length) {
			const sub = command.subcommands.find((s) => s.name === parts[idx]);
			if (!sub) break;
			command = sub;
			consumed += 1 + sub.name.length;
			idx++;
		}

		return { command, consumed };
	}

	parse(
		text: string,
	): { command: Command; args: Record<string, unknown> } | undefined {
		const match = this.match(text);
		if (!match) return undefined;

		const rawArgs = tokenizeArgs(text.slice(match.consumed)).map(
			(token) => token.value,
		);

		// TODO: parse options from parent command?
		return {
			command: match.command,
			args: parseOptions(match.command.options, rawArgs),
		};
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

		resolveUserArgs(api, channel, parsed.command.options, parsed.args);

		parsed.command.execute(
			api,
			channel,
			parsed.args as any, // TODO: better typing
		);
	}
}

/** commands expect user ids, but users may type the displayed name */
function resolveUserArgs(
	api: RootStore,
	channel: Channel,
	options: OptionDef[],
	args: Record<string, unknown>,
) {
	for (const opt of options) {
		if (opt.kind !== "user") continue;

		const value = args[opt.name];
		if (typeof value !== "string" || !value) continue;

		const mention = /^<@([^>]+)>$/.exec(value);
		if (mention) {
			args[opt.name] = mention[1];
			continue;
		}

		if (api.users.cache.get(value)) continue;

		const user = [...api.users.cache.values()].find((u) => u.name === value);
		if (user) {
			args[opt.name] = user.id;
			continue;
		}

		const room_id = channel.room_id;
		if (!room_id) continue;

		const member = [...api.roomMembers.cache.values()].find(
			(m) => m.room_id === room_id && m.override_name === value,
		);
		if (member) args[opt.name] = member.user_id;
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
				rawArgs[idx] !== undefined ? parseDuration(rawArgs[idx]) : undefined;
			idx++;
		} else {
			args[opt.name] = rawArgs[idx];
			idx++;
		}
	});
	return args;
}
