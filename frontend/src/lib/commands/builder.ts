import { ChannelT } from "@/types";
import { ChannelType, Permission } from "ts-sdk";
import { Args, Command, OptionDef, OptionKind } from "./types";
import { createPermissionChecker } from "@/lib/permissions/calculator";
import { Api } from "@/api";

export type CommandBuilder<O extends OptionDef[]> = {
	name(name: string): CommandBuilder<O>;
	description(desc: string): CommandBuilder<O>;
	option<Kind extends OptionKind, B extends OptionDef>(
		type: Kind,
		fn: (
			b: OptionBuilder<{ name: ""; kind: Kind; required: false }>,
		) => OptionBuilder<B>,
	): CommandBuilder<[...O, B]>;
	requires(fn: (b: RequiresBuilder) => RequiresBuilder): CommandBuilder<O>;
	subcommand(
		name: string,
		fn: (b: CommandBuilder<[]>) => Command,
	): CommandBuilder<O>;
	executes(fn: (args: Args<O>) => void): Command;
};

type BuilderState<O extends OptionDef[]> = {
	id: string;
	name: string;
	description: string;
	options: O;
	permissions: Permission[][];
	channelTypes: ChannelType[];
	insideRoom: boolean;
	subcommands: Command[];
};

export type OptionBuilder<Def extends OptionDef> = {
	name<N extends string>(
		name: N,
	): OptionBuilder<Omit<Def, "name"> & { name: N }>;
	description(desc: string): OptionBuilder<Def>;
	required(): OptionBuilder<Omit<Def, "required"> & { required: true }>;
	__def: Def;
};

export type RequiresBuilder = {
	/** can only be used if user has one of these permissions */
	permission(...perms: Permission[]): RequiresBuilder;

	/** can only be used in a channel with one of these types */
	channelType(...types: ChannelType[]): RequiresBuilder;

	// TODO: channelIsThread()

	/** can only be used inside a room */
	// TODO: update types so that if insideRoom() is called Args always has `room`
	insideRoom(): RequiresBuilder;

	__permissions: Permission[][];
	__channelTypes: ChannelType[];
	__insideRoom: boolean;
};

function createOptionBuilder<Def extends OptionDef>(
	def: Def,
): OptionBuilder<Def> {
	return {
		name: (name) => createOptionBuilder({ ...def, name }),
		description: (description) => createOptionBuilder({ ...def, description }),
		required: () => createOptionBuilder({ ...def, required: true }),
		__def: def,
	};
}

function createRequiresBuilder(state: {
	permissions: Permission[][];
	channelTypes: ChannelType[];
	insideRoom: boolean;
}): RequiresBuilder {
	return {
		permission: (...perms) =>
			createRequiresBuilder({
				...state,
				permissions: [...state.permissions, perms],
			}),
		channelType: (...types) =>
			createRequiresBuilder({
				...state,
				channelTypes: [...state.channelTypes, ...types],
			}),
		insideRoom: () =>
			createRequiresBuilder({
				...state,
				insideRoom: true,
			}),
		__permissions: state.permissions,
		__channelTypes: state.channelTypes,
		__insideRoom: state.insideRoom,
	};
}

function createCommandBuilder<O extends OptionDef[]>(
	state: BuilderState<O>,
): CommandBuilder<O> {
	const canUse = (api: Api, channel: ChannelT) => {
		// check channel type
		if (state.channelTypes.length && !state.channelTypes.includes(channel.type))
			return false;

		// check room
		if (state.insideRoom && !channel.room_id) return false;

		// check permissions
		if (state.permissions.length) {
			const self_id = api.users.cache.get("@self")?.id;
			if (!self_id) return false;

			const perms = createPermissionChecker(
				{ api, room_id: channel.room_id ?? undefined, channel_id: channel.id },
				self_id,
			);

			return state.permissions.every((ps) => ps.some((p) => perms.has(p)));
		}

		// check that any subcommand can be run
		if (state.subcommands.length) {
			return state.subcommands.some((s) => s.canUse(api, channel));
		}

		return true;
	};

	return {
		name: (name) => createCommandBuilder({ ...state, name }),
		description: (description) =>
			createCommandBuilder({ ...state, description }),

		option: (type, fn) => {
			const built = fn(
				createOptionBuilder({ name: "", kind: type, required: false }),
			);
			return createCommandBuilder({
				...state,
				options: [...state.options, built.__def] as any,
			});
		},

		requires: (fn) => {
			const built = fn(
				createRequiresBuilder({
					permissions: state.permissions,
					channelTypes: state.channelTypes,
					insideRoom: state.insideRoom,
				}),
			);
			return createCommandBuilder({
				...state,
				permissions: built.__permissions,
				channelTypes: built.__channelTypes,
				insideRoom: built.__insideRoom,
			});
		},

		subcommand: (name, fn) => {
			const sub = fn(
				createCommandBuilder({
					id: `${state.id}.${name}`,
					name,
					description: "",
					options: [],
					permissions: [],
					channelTypes: [],
					subcommands: [],
					insideRoom: false,
				}),
			);
			return createCommandBuilder({
				...state,
				subcommands: [...state.subcommands, sub],
			});
		},

		executes: (execFn) => ({
			id: state.id,
			name: state.name,
			description: state.description,
			options: state.options.map((o) => ({
				name: o.name,
				description: o.description ?? "",
				kind: o.kind,
				required: o.required,
			})),
			subcommands: state.subcommands,

			canUse,

			execute: async (api, channel, args) => {
				if (!canUse(api, channel)) {
					// TODO: better error reporting
					console.error(`Insufficient permissions to use /${state.name}.`);
					return;
				}

				const room_id = channel.room_id;
				const room = room_id ? api.rooms.cache.get(room_id) : undefined;

				execFn({
					channel,
					room,
					args,
				});
			},
		}),
	};
}

export const command = (id: string): CommandBuilder<[]> =>
	createCommandBuilder({
		id,
		name: id,
		description: "",
		options: [],
		permissions: [],
		channelTypes: [],
		subcommands: [],
		insideRoom: false,
	});
