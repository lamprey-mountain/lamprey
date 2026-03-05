
async function startDb() {
    console.log("Starting database with Docker Compose...");
    const cmd = new Deno.Command("docker", {
        args: ["compose", "up", "-d", "postgres"],
    });
    const { success } = await cmd.output();
    if (!success) {
        console.error("Failed to start database with Docker Compose.");
        Deno.exit(1);
    }

    console.log("Waiting for database health check...");
    let ready = false;
    for (let i = 0; i < 30; i++) {
        const cmd = new Deno.Command("docker", {
            args: ["inspect", "--format", "{{.State.Health.Status}}", "lamprey-postgres"],
        });
        const { stdout } = await cmd.output();
        const status = new TextDecoder().decode(stdout).trim();
        if (status === "healthy") {
            ready = true;
            break;
        }
        await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    if (!ready) {
        console.error("Database failed to become healthy.");
        Deno.exit(1);
    }
    console.log("Database is ready.");
}

async function runBackend() {
    console.log("Starting backend...");
    const cmd = new Deno.Command("cargo", {
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
        stdout: "inherit",
        stderr: "inherit",
    });

    const process = cmd.spawn();
    const status = await process.status;
    if (!status.success) {
        console.error("Backend exited with error code:", status.code);
        Deno.exit(status.code);
    }
}

if (import.meta.main) {
    await Deno.mkdir("data/blobs", { recursive: true });
    await startDb();
    await runBackend();
}
