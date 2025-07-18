import { For, type VoidProps } from "solid-js";
import { type User } from "sdk";
import { useApi } from "../api.tsx";
import { createResource } from "solid-js";
import { useCtx } from "../context.ts";

export function Email(_props: VoidProps<{ user: User }>) {
	const api = useApi();
	const ctx = useCtx();

	const [emails, { refetch }] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/user/{user_id}/email", {
			params: { path: { user_id: "@self" } },
		});
		return data;
	});

	function addEmail() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "email?",
			cont(email) {
				if (!email) return;
				api.client.http.PUT("/api/v1/user/{user_id}/email/{addr}", {
					params: { path: { user_id: "@self", addr: email } },
				}).then(refetch);
			},
		});
	}

	function deleteEmail(email: string) {
		ctx.dispatch({
			do: "modal.confirm",
			text: "delete email?",
			cont(conf) {
				if (!conf) return;
				api.client.http.DELETE("/api/v1/user/{user_id}/email/{addr}", {
					params: { path: { user_id: "@self", addr: email } },
				}).then(refetch);
			},
		});
	}

	function resendVerification(email: string) {
		api.client.http.POST(
			"/api/v1/user/{user_id}/email/{addr}/resend-verification",
			{
				params: { path: { user_id: "@self", addr: email } },
			},
		);
	}

	return (
		<>
			<h2>email</h2>
			<For each={emails()}>
				{(email) => (
					<div>
						{email.email}{" "}
						{email.is_verified ? "(verified)" : "(unverified)"}
						<button onClick={() => deleteEmail(email.email)}>delete</button>
						{!email.is_verified && (
							<button onClick={() => resendVerification(email.email)}>
								resend verification
							</button>
						)}
					</div>
				)}
			</For>
			<button onClick={addEmail}>add email</button>
		</>
	);
}
