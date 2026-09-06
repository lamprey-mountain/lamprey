import type { ApiError } from "sdk";
import { Icon } from "@/atoms/Icon";
import { icWarning } from "@/utils/icons";

export const isApiError = (err: unknown): err is ApiError => {
	if (typeof err !== "object") return false;
	if (!err) return false;
	if (!("code" in err)) return false;
	if (!("message" in err)) return false;
	return true;
};

export const renderError = (code: string) => {
	switch (code) {
		case "UnknownUser":
			return "User not found";
		case "UnknownInvite":
			return "Invite code not found";
		default:
			return code;
	}
};

export const ApiErrorDisplay = (props: { error: ApiError }) => (
	<div class="error-container">
		<div class="error-message">
			<Icon src={icWarning} />
			<div>
				<div>
					<span class="error-prefix">Error:</span>{" "}
					{renderError(props.error.code)}
				</div>
				<div class="error-code">{props.error.code}</div>
			</div>
		</div>
	</div>
);
