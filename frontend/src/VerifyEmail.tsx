import { createSignal, type JSX, Show } from "solid-js";
import { RouteSectionProps, useNavigate } from "@solidjs/router";
import { useCtx } from "./context.ts";

export const RouteVerifyEmail = (
	props: RouteSectionProps<unknown>,
): JSX.Element => {
	const ctx = useCtx();
	const nav = useNavigate();
	const [error, setError] = createSignal<string | null>(null);
	const [verifying, setVerifying] = createSignal(false);
	const email = () => props.location.query.email;

	const verify = () => {
		const code = props.location.query.code;
		const emailAddr = email();

		if (!emailAddr || !code) {
			setError("Email or code missing from verification link.");
			return;
		}

		// Handle potential array values from query params
		const addrStr = Array.isArray(emailAddr) ? emailAddr[0] : emailAddr;
		const codeStr = Array.isArray(code) ? code[0] : code;

		setVerifying(true);
		setError(null);

		ctx.client.http
			.POST("/api/v1/user/{user_id}/email/{addr}/verify/{code}", {
				params: {
					path: {
						addr: addrStr,
						code: codeStr,
						user_id: "@self",
					},
				},
			})
			.then(({ error }) => {
				if (error) {
					console.error(error);
					setError(`Failed to verify email: ${String(error)}`);
				} else {
					nav("/settings/email");
				}
			})
			.catch((err) => {
				console.error(err);
				setError(`An unexpected error occurred: ${err.message}`);
			})
			.finally(() => {
				setVerifying(false);
			});
	};

	return (
		<div>
			<p>Verify email address: {email() ?? "(email not found in link)"}</p>
			<br />
			<button onClick={verify} disabled={verifying() || !email()}>
				<Show when={verifying()} fallback={"Verify"}>
					Verifying...
				</Show>
			</button>
			<Show when={error()}>
				<p>There was an error verifying your email address:</p>
				<pre>{error()}</pre>
			</Show>
		</div>
	);
};
