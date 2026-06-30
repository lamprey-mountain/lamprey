export function highlight(el: Element) {
	el.animate(
		[
			{
				boxShadow: "4px 0 0 -1px inset oklch(var(--color-highlight))",
				backgroundColor: "oklch(var(--color-highlight) / 0.15)",
				offset: 0,
			},
			{
				boxShadow: "4px 0 0 -1px inset oklch(var(--color-highlight))",
				backgroundColor: "oklch(var(--color-highlight) / 0.15)",
				offset: 0.8,
			},
			{
				boxShadow: "none",
				backgroundColor: "transparent",
				offset: 1,
			},
		],
		{
			duration: 1000,
		},
	);
}
