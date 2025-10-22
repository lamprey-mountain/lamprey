const crates = ["crate-backend", "crate-bridge", "crate-media"];

for (const crate of crates) {
	const process = Deno.run({
		cmd: ["cargo", "sqlx", "prepare"],
		cwd: crate,
		stdout: "inherit",
		stderr: "inherit",
	});
	const status = await process.status();
	process.close();

	if (!status.success) {
		console.error(`cargo sqlx prepare failed for ${crate}`);
		Deno.exit(1);
	}
}

console.log("done preparing sqlx queries");
