import type { VoidProps } from "solid-js";

export type IconProps = {
	src: string;
	color?: string;
};

const DEFAULT_ICON_COLOR = "oklch(var(--color-fg4))";

export const Icon = (props: VoidProps<IconProps>) => {
	return (
		<div
			class="icon2"
			style={{
				"mask-image": `url(${props.src})`,
				background: props.color ?? DEFAULT_ICON_COLOR,
			}}
		/>
	);
};
