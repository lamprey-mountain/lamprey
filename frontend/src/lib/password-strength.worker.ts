import zxcvbn from "zxcvbn";
import type {
	PasswordStrengthRequest,
	PasswordStrengthResponse,
} from "./password-strength";

declare const self: {
	onmessage: ((event: MessageEvent<PasswordStrengthRequest>) => void) | null;
	postMessage: (message: PasswordStrengthResponse) => void;
};

// zxcvbn scoring is expensive for long passwords, so it runs in a worker to
// keep the main thread responsive while typing.
self.onmessage = (event) => {
	const { id, password, inputs } = event.data;
	const result = zxcvbn(password, inputs);
	self.postMessage({
		id,
		strength: {
			score: result.score,
			warning: result.feedback.warning,
			suggestions: result.feedback.suggestions,
		},
	});
};
