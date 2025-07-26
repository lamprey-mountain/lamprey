import { RouteSectionProps, useNavigate } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";

export const RouteVerifyEmail = (
	props: RouteSectionProps<unknown>,
) => {
	const ctx = useCtx();
	const nav = useNavigate();

	const verify = () => {
		ctx.client.http.POST("/api/v1/user/{user_id}/email/{addr}/verify/{code}", {
			params: {
				path: {
					addr: props.location.query.email,
					code: props.location.query.code,
					user_id: "@self",
				},
			},
		}).then(({ error }) => {
			if (error) {
				console.error(error);
				alert("error while verifying, see console");
			} else {
				nav("/settings/email");
			}
		}).catch((err) => {
			console.error(err);
			alert("error while verifying, see console");
		});
	};

	return (
		<div>
			verify email address?
			<br />
			<button onClick={verify}>verify</button>
		</div>
	);
};
