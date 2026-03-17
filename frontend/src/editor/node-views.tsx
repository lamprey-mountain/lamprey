import { render } from "solid-js/web";

export const createSolidNodeView = (
	Component: any,
	propsFn: (node: any) => any,
) => {
	return (node: any) => {
		const dom = document.createElement("span");
		dom.classList.add("mention-wrapper");

		const dispose = render(() => <Component {...propsFn(node)} />, dom);

		return {
			dom,
			destroy: () => dispose(),
		};
	};
};
