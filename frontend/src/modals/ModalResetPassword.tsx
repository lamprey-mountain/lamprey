import {
	createMemo,
	createSignal,
	For,
	onCleanup,
	type ParentProps,
	Show,
} from "solid-js";
import { useApi } from "@/api";
import { CheckboxOptionWithLabel } from "@/atoms/CheckboxOption";
import { ToggleIcon } from "@/atoms/ToggleIcon";
import { createTooltip } from "@/atoms/Tooltip";
import { useModals } from "@/contexts/modal";
import {
	createPasswordStrengthScorer,
	type PasswordStrength,
} from "@/lib/password-strength";
import { icEye } from "@/utils/icons";
import { Modal } from "./mod";

const STRENGTH_LABELS = [
	"very weak",
	"weak",
	"fair",
	"strong",
	"very strong",
] as const;

const STRENGTH_CLASSES = [
	"very-weak",
	"weak",
	"fair",
	"strong",
	"very-strong",
] as const;

export const ModalResetPassword = () => {
	const [password, setPassword] = createSignal("");
	const [confirmPassword, setConfirmPassword] = createSignal("");
	const [strength, setStrength] = createSignal<PasswordStrength | null>(null);
	const [checking, setChecking] = createSignal(false);
	const [checkError, setCheckError] = createSignal(false);
	const [allowInsecure, setAllowInsecure] = createSignal(false);
	const [saving, setSaving] = createSignal(false);
	const [submitError, setSubmitError] = createSignal<string | null>(null);
	const [showPassword, setShowPassword] = createSignal(false);
	const api = useApi();
	const [, modalctl] = useModals();
	const showPasswordTooltip = createTooltip({
		tip: () => (showPassword() ? "Hide password" : "Show password"),
	});

	const scorer = createPasswordStrengthScorer();
	let latestRequestId = 0;

	onCleanup(() => scorer.dispose());

	const tooWeak = createMemo(() => {
		// if the strength couldn't be determined, require explicitly allowing
		// an insecure password instead of silently letting it through
		if (checkError()) return true;
		const result = strength();
		return result != null && result.score < 2;
	});

	const canSubmit = createMemo(
		() =>
			!checking() &&
			!saving() &&
			password() !== "" &&
			confirmPassword() !== "" &&
			password() === confirmPassword() &&
			(!tooWeak() || allowInsecure()),
	);

	const confirmError = createMemo(() => {
		if (password() !== "" && confirmPassword() === "")
			return "confirm your password";
		if (confirmPassword() !== "" && password() !== confirmPassword())
			return "passwords don't match";
		return null;
	});

	/** user-specific terms zxcvbn should treat as weak */
	const userInputs = () => {
		const session = api.session();
		if (!session || session.status === "Unauthorized") return [];
		const me = api.users.get(session.user_id);
		if (!me) return [];
		return [me.name, ...(me.emails ?? []).map((email) => email.email)].filter(
			(input) => input.length > 0,
		);
	};

	async function evaluate(value: string) {
		const id = ++latestRequestId;

		setCheckError(false);
		if (!value) {
			setStrength(null);
			setChecking(false);
			return;
		}

		// the previous result stays visible (and submit stays disabled via
		// `checking`) so the bar can transition to the new score
		setChecking(true);
		const result = await scorer.score(value, userInputs());
		if (latestRequestId !== id) return;

		if (result) {
			setStrength(result);
		} else {
			setCheckError(true);
		}
		setChecking(false);
	}

	async function handlePasswordSet(e: SubmitEvent) {
		e.preventDefault();

		if (!canSubmit()) return;

		setSubmitError(null);
		setSaving(true);
		try {
			const { error } = await api.client.http.PUT("/api/v1/auth/password", {
				body: { password: password() },
			});
			if (error) throw error;
			modalctl.close();
		} catch (err) {
			console.error("failed to set password:", err);
			setSubmitError("failed to set password");
		} finally {
			setSaving(false);
		}
	}

	return (
		<Modal
			class="reset-password"
			onKeyDown={(e) => {
				if (e.key === "Escape") modalctl.close();
			}}
		>
			<form onSubmit={handlePasswordSet}>
				<h3>change password</h3>
				<label class="field">
					<span class="field-label dim">password</span>
					<div class="password-input">
						<input
							type={showPassword() ? "text" : "password"}
							name="password"
							autocomplete="new-password"
							placeholder="dolphins"
							value={password()}
							onInput={(e) => {
								setSubmitError(null);
								setPassword(e.currentTarget.value);
								void evaluate(e.currentTarget.value);
							}}
							ref={(el) => queueMicrotask(() => el.focus())}
						/>
						<button
							type="button"
							class="button icon-button"
							ref={showPasswordTooltip.content}
							aria-label={showPassword() ? "hide password" : "show password"}
							aria-pressed={showPassword()}
							onClick={() => setShowPassword((shown) => !shown)}
						>
							<ToggleIcon
								enabled={showPassword()}
								src={icEye}
								alt="show password"
							/>
						</button>
					</div>
					<Show when={checking() && !strength()}>
						<div class="checking dim">checking strength…</div>
					</Show>
					<Show when={strength()}>{(s) => <Strength result={s} />}</Show>
					<Show when={checkError()}>
						<FieldError>couldn't check password strength</FieldError>
					</Show>
				</label>
				<div class="field">
					<span class="field-label dim">confirm password</span>
					<input
						type={showPassword() ? "text" : "password"}
						name="confirm-password"
						autocomplete="new-password"
						placeholder="dolphins"
						value={confirmPassword()}
						onInput={(e) => {
							setSubmitError(null);
							setConfirmPassword(e.currentTarget.value);
						}}
					/>
					<Show when={tooWeak()}>
						<CheckboxOptionWithLabel
							id="allow-insecure"
							seed="allow-insecure"
							label="allow insecure password"
							checked={allowInsecure()}
							onChange={setAllowInsecure}
							background="oklch(var(--color-yellow))"
						/>
					</Show>
					<Show when={confirmError()}>
						{(m) => <FieldError>{m()}</FieldError>}
					</Show>
				</div>
				<Show when={submitError()}>
					{(m) => <FieldError>{m()}</FieldError>}
				</Show>
				<div class="bottom">
					<button
						type="button"
						class="button link link-dim"
						onClick={modalctl.close}
					>
						cancel
					</button>
					<input
						type="submit"
						class="button primary"
						value="set password"
						disabled={!canSubmit()}
					/>
				</div>
			</form>
		</Modal>
	);
};

function FieldError(props: ParentProps) {
	return <div class="field-error">{props.children}</div>;
}

function Strength(props: { result: () => PasswordStrength }) {
	const strengthClass = () => STRENGTH_CLASSES[props.result().score];
	const label = () => STRENGTH_LABELS[props.result().score];
	const warnings = createMemo(() => {
		const { warning, suggestions } = props.result();
		return [warning, ...suggestions].filter((w) => w.length > 0);
	});
	const fill = () => ((props.result().score + 1) / 5) * 100;
	return (
		<div class="password-strength">
			<div class="meter">
				<div class="track">
					<div
						class={`fill ${strengthClass()}`}
						style={{ width: `${fill()}%` }}
					/>
				</div>
				<div class={`label ${strengthClass()}`}>{label()}</div>
			</div>
			<Show when={warnings().length > 0}>
				<ul class="warnings dim">
					<For each={warnings()}>{(warning) => <li>{warning}</li>}</For>
				</ul>
			</Show>
		</div>
	);
}
