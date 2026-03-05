if (import.meta.main) {
	console.log("DATABASE_URL:", Deno.env.get("DATABASE_URL"));
	try {
		const command = new Deno.Command("psql", {
			args: [Deno.env.get("DATABASE_URL") || "", "-c", "SELECT 1"],
		});
		const { code, stdout, stderr } = await command.output();
		console.log("psql exit code:", code);
		console.log("stdout:", new TextDecoder().decode(stdout));
		console.log("stderr:", new TextDecoder().decode(stderr));
	} catch (e) {
		console.error("Failed to run psql:", e);
	}
}
