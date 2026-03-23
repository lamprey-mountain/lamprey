export const ToggleIcon = (props: { checked: boolean; src: string }) => {
	return (
		<svg
			viewBox={`0 0 64 64`}
			role="img"
			class="icon strike"
			aria-checked={props.checked}
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
			<image href={props.src} />
			<line class="line" x1="8" y1="8" x2="56" y2="56" stroke-width="8" />
		</svg>
	);
};
