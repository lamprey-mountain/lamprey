import type { VoidProps } from "solid-js";
import { colors } from "@/lib/colors";

export type IconProps = {
	src: string;
	alt?: string;
	color?: string | null;
};

const DEFAULT_ICON_COLOR = colors.fg400;

export const Icon = (props: VoidProps<IconProps>) => {
	return (
		<div
			class="icon2"
			role="img"
			aria-label={props.alt}
			style={{
				"mask-image": `url(${props.src})`,
				// TODO: deprecate and remove props.color/--icon-color
				"--icon-color":
					props.color === null
						? undefined
						: (props.color ?? DEFAULT_ICON_COLOR),
			}}
		/>
	);
};
