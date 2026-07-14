import { Api } from "@/api";
import { ChannelT, RoomT } from "@/types";

export type OptionKind = "string" | "user" | "duration";

export type OptionDef = {
	name: string;
	kind: OptionKind;
	description?: string;
	required: boolean;
};

export type Command = {
	id: string;
	name: string;
	description: string;
	options: OptionDef[];
	subcommands: Command[];

	// TODO: clean up these types?
	canUse: (api: Api, channel: ChannelT) => boolean;
	execute: (api: Api, channel: ChannelT, args: OptionsToArgs<any>) => void;
};

export type Args<O extends OptionDef[]> = {
	channel: ChannelT;
	room?: RoomT;
	args: OptionsToArgs<O>;
};

export type KindToType<K extends OptionKind> = K extends "string"
	? string
	: K extends "user"
		? string
		: K extends "duration"
			? number
			: never;

export type OptionsToArgs<O extends OptionDef[]> = {
	[K in O[number] as K["name"]]: K["required"] extends true
		? KindToType<K["kind"]>
		: KindToType<K["kind"]> | undefined;
};
