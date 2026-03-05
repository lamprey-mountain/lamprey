import { delay } from "https://deno.land/std@0.208.0/async/delay.ts";

async function runTestHarness() {
	let backendProcess: Deno.ChildProcess | null = null;

	try {
		// 0. Kill existing backend if any
		try {
			const ssCmd = new Deno.Command("ss", { args: ["-tulpn"] });
			const { stdout } = await ssCmd.output();
			const output = new TextDecoder().decode(stdout);
			const match = output.match(/:4000.*pid=(\d+)/);
			if (match) {
				const pid = match[1];
				console.log(`Killing existing backend process with PID ${pid}...`);
				await new Deno.Command("kill", { args: ["-9", pid] }).output();
			}
		} catch {}

		// 1. Ensure DB is up
		console.log("Setting up database for tests...");
		await new Deno.Command("docker", {
			args: ["compose", "up", "-d", "postgres"],
		}).output();

		// 1.5 Build backend
		console.log("Building backend...");
		const buildCmd = new Deno.Command("cargo", {
			args: ["build", "-p", "lamprey-backend", "--bin", "lamprey"],
			stdout: "inherit",
			stderr: "inherit",
		});
		const buildStatus = await buildCmd.output();
		if (!buildStatus.success) {
			throw new Error("Failed to build backend.");
		}

		// Wait for DB to be really ready
		await delay(5000);

		// 2. Start Backend in background
		console.log("Starting backend for tests...");
		backendProcess = new Deno.Command("cargo", {
			args: [
				"run",
				"-p",
				"lamprey-backend",
				"--bin",
				"lamprey",
				"--",
				"--config",
				"config.dev.toml",
				"serve",
			],
			// ...
			env: {
				"RUST_LOG": "info,lamprey=debug,tower_http=debug",
			},
			stdout: "inherit",
			stderr: "inherit",
		}).spawn();

		// 3. Wait for Backend to be ready
		console.log(
			"Waiting for backend to be ready (this may take a while on the first run)...",
		);
		let ready = false;
		for (let i = 0; i < 60; i++) {
			try {
				const res = await fetch("http://localhost:4000/api/v1/health");
				if (res.ok) {
					ready = true;
					break;
				}
			} catch {}
			await delay(1000);
		}

		if (!ready) {
			throw new Error("Backend failed to become ready for tests.");
		}

		// 4. Run Deno Tests
		console.log("Running integration tests...");
		const testCmd = new Deno.Command("deno", {
			args: [
				"test",
				"-A",
				"tests/",
			],
			env: {
				"BASE_URL": "http://localhost:4000",
			},
			stdout: "inherit",
			stderr: "inherit",
		});

		const { code } = await testCmd.output();
		if (code !== 0) {
			throw new Error(`Tests failed with code ${code}`);
		}
		console.log("Tests passed!");
	} catch (e) {
		console.error(e.message);
		Deno.exit(1);
	} finally {
		console.log("Cleaning up...");
		if (backendProcess) {
			try {
				backendProcess.kill();
			} catch {}
		}
		// Shut down DB and remove volumes to ensure next run is fresh
		await new Deno.Command("docker", {
			args: ["compose", "down", "-v"],
		}).output();
	}
}

if (import.meta.main) {
	await runTestHarness();
}
