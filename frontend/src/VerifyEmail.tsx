import { RouteSectionProps } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";

export const RouteVerifyEmail = (
	props: RouteSectionProps<unknown>,
) => {
	const ctx = useCtx();

	const verify = () => {
		console.log("verify email addr", props.location.query.code);
		alert("todo");
	};

	return (
		<div>
			verify email address?
			<br />
			<button onClick={verify}>verify</button>
		</div>
	);
};
