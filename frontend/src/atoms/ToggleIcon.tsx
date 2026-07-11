export type ToggleIconProps = {
	src: string;
	alt?: string;
	color?: string | null;
	checked?: boolean;
};

export const ToggleIcon = (props: ToggleIconProps) => {
	return (
		<svg
			aria-hidden="true"
			viewBox={`0 0 64 64`}
			role="img"
			class="toggle-icon"
			aria-checked={props.checked}
			aria-label={props.alt}
		>
			<defs>
				<mask id="strike">
					<rect width="64" height="64" fill="white" />
					<line
						x1="0"
						y1="0"
						x2="64"
						y2="64"
						stroke="black"
						stroke-width="32"
					/>
				</mask>
			</defs>
			<g class="icon-wrap">
				<rect
					height="64"
					width="64"
					class="icon-background"
					style={{
						"mask-image": `url(${props.src})`,
					}}
				/>
			</g>
			<line class="line" x1="8" y1="8" x2="56" y2="56" />
		</svg>
	);
};
