import { onMount } from "solid-js";

/** directive to focus a component (html element only for now) on mount */
export function autofocus(el: HTMLElement) {
	onMount(() => el.focus());
}
