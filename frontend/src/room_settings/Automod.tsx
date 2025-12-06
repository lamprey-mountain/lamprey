import { Room } from "sdk";
import { VoidProps } from "solid-js";

export function Automod(props: VoidProps<{ room: Room }>) {
	// TODO: {create,list,edit,delete} automod rules
	// TODO: configuring nsfw scanning
	// NOTE: maybe rename this to something more generic (eg. "Moderation?")

	return (
		<div>
			<h2>automod</h2>
		</div>
	);
}
