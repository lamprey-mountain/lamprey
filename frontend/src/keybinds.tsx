export function createKeybinds(
	binds: Record<string, (e: KeyboardEvent) => void>,
): (e: KeyboardEvent) => void {
	type Bind = {
		ctrl: boolean;
		alt: boolean;
		shift: boolean;
		key: string;
	};
	type Chord = Array<Bind>;

	const realBinds: Array<[Chord, (e: KeyboardEvent) => void]> = [];
	for (const bind in binds) {
		for (const version of bind.split(",")) {
			const keys = version.trim().split(" ").map((i) => i.split("-"));
			const chord = [];
			for (const key of keys) {
				chord.push({
					ctrl: key.includes("Ctrl"),
					shift: key.includes("Shift"),
					alt: key.includes("Alt"),
					key: key[key.length - 1]!,
				});
			}
			realBinds.push([chord, binds[bind]]);
		}
	}
	let valid: typeof realBinds = [];
	return (e) => {
		if (["Shift", "Ctrl", "Alt"].includes(e.key)) return;
		valid = [...valid, ...realBinds];
		const newValid: typeof realBinds = [];
		for (const [chord, call] of valid) {
			const bind = chord[0];
			if (
				e.ctrlKey === bind.ctrl && e.shiftKey === bind.shift &&
				e.altKey === bind.alt && e.key === bind.key
			) {
				if (chord.length === 1) {
					e.stopPropagation();
					call(e);
				} else {
					newValid.push([chord.slice(1), call]);
				}
			}
		}
		valid = newValid;
	};
}
