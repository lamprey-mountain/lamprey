import { createEffect } from "solid-js";

export const Title = (props: { title?: string }) => {
	createEffect(() => {
		document.title = props.title ?? "";
	});
	return undefined;
};
