import { useMenu } from "@/contexts/mod.tsx";
import { Item, Menu, Separator } from "./Parts.tsx";

type ColorPickerOptionsProps = {
	onColorSpaceChange: (space: "oklch" | "srgb") => void;
};

export function ColorPickerOptions(props: ColorPickerOptionsProps) {
	const { setMenu } = useMenu();

	return (
		<Menu>
			<h3 class="dim" style="margin:2px 8px">
				Colorspace
			</h3>
			<Item
				onClick={() => {
					props.onColorSpaceChange("oklch");
					setMenu(null);
				}}
			>
				OKLCH
			</Item>
			<Item
				onClick={() => {
					props.onColorSpaceChange("srgb");
					setMenu(null);
				}}
			>
				sRGB
			</Item>
			{/* NOTE: maybe have preset colors here?
			<Separator />
			<h3 class="dim" style="margin:2px 8px">Presets</h3>
			<Item><div style="display:inline-block;height:1em;width:1em;margin-right:4px;vertical-align:center;background:oklch(var(--color-red));"></div> red</Item>
			<Item><div style="display:inline-block;height:1em;width:1em;margin-right:4px;vertical-align:center;background:oklch(var(--color-orange));"></div> orange</Item>
			<Item><div style="display:inline-block;height:1em;width:1em;margin-right:4px;vertical-align:center;background:oklch(var(--color-yellow));"></div> yellow</Item>
			<Item><div style="display:inline-block;height:1em;width:1em;margin-right:4px;vertical-align:center;background:oklch(var(--color-green));"></div> green</Item>
			<Item><div style="display:inline-block;height:1em;width:1em;margin-right:4px;vertical-align:center;background:oklch(var(--color-teal));"></div> teal</Item>
			<Item><div style="display:inline-block;height:1em;width:1em;margin-right:4px;vertical-align:center;background:oklch(var(--color-cyan));"></div> cyan</Item>
			<Item><div style="display:inline-block;height:1em;width:1em;margin-right:4px;vertical-align:center;background:oklch(var(--color-blue));"></div> blue</Item>
			<Item><div style="display:inline-block;height:1em;width:1em;margin-right:4px;vertical-align:center;background:oklch(var(--color-magenta));"></div> magenta</Item>
		*/}
		</Menu>
	);
}
