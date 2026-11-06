import { base64UrlToBytes, bytesToBase64Url } from "./base64.ts";

/**
 * the ceremony options as serialized by the backend (webauthn-rs): the dom
 * shape, but with the binary fields as unpadded url safe base64
 */
interface CreationOptionsJson {
	publicKey: Omit<
		PublicKeyCredentialCreationOptions,
		"challenge" | "user" | "excludeCredentials"
	> & {
		challenge: string;
		user: Omit<PublicKeyCredentialUserEntity, "id"> & { id: string };
		excludeCredentials?: (Omit<PublicKeyCredentialDescriptor, "id"> & {
			id: string;
		})[];
	};
}

/** the response body the backend expects for `register/finish` */
interface AttestationResponseJson {
	id: string;
	rawId: string;
	type: string;
	response: {
		clientDataJSON: string;
		attestationObject: string;
	};
}

export function isWebauthnSupported(): boolean {
	return typeof window !== "undefined" && "PublicKeyCredential" in window;
}

/** a user facing message for a failed ceremony */
export function webauthnErrorMessage(err: unknown): string {
	if (err instanceof DOMException) {
		switch (err.name) {
			case "NotAllowedError":
				return "cancelled or timed out";
			case "InvalidStateError":
				return "this passkey is already registered";
			case "NotSupportedError":
				return "this device does not support passkeys";
			case "SecurityError":
				return "this domain is not allowed to use passkeys";
		}
	}

	if (isApiError(err)) {
		switch (err.code) {
			case "InvalidOrExpiredCode":
				return "the passkey challenge expired, try again";
			case "UnknownAuthenticator":
				return "unknown passkey";
			case "CannotRemoveLastAuthMethod":
				return "this is your only way to log in";
		}
		return err.code;
	}

	if (err instanceof Error) return err.message;
	return "something went wrong";
}

/** run a registration ceremony and serialize the result for the backend */
export async function createPasskey(challenge: string): Promise<string> {
	const credential = await navigator.credentials.create({
		publicKey: parseCreationOptions(challenge),
	});

	if (!(credential instanceof PublicKeyCredential)) {
		throw new Error("no passkey was created");
	}

	if (!(credential.response instanceof AuthenticatorAttestationResponse)) {
		throw new Error("unexpected passkey response");
	}

	const response: AttestationResponseJson = {
		id: credential.id,
		rawId: bytesToBase64Url(new Uint8Array(credential.rawId)),
		type: credential.type,
		response: {
			clientDataJSON: bytesToBase64Url(
				new Uint8Array(credential.response.clientDataJSON),
			),
			attestationObject: bytesToBase64Url(
				new Uint8Array(credential.response.attestationObject),
			),
		},
	};

	return JSON.stringify(response);
}

function isApiError(err: unknown): err is { code: string } {
	return (
		typeof err === "object" &&
		err !== null &&
		"code" in err &&
		typeof err.code === "string"
	);
}

function parseCreationOptions(
	challenge: string,
): PublicKeyCredentialCreationOptions {
	const { publicKey }: CreationOptionsJson = JSON.parse(challenge);

	return {
		...publicKey,
		challenge: base64UrlToBytes(publicKey.challenge),
		user: { ...publicKey.user, id: base64UrlToBytes(publicKey.user.id) },
		excludeCredentials: publicKey.excludeCredentials?.map((descriptor) => ({
			...descriptor,
			id: base64UrlToBytes(descriptor.id),
		})),
	};
}
