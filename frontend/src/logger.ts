export const colors = {
	magenta: "oklch(80.6% 0.15 299.2)",
	red: "oklch(74.03% 0.1759 13.16)",
	yellow: "oklch(85.39% 0.1187 92.43)",

	// reserved for api and service layer
	orange: "oklch(80.7% 0.1273 50.56)",

	green: "oklch(85.53% 0.1395 130.14)",
	cyan: "oklch(80.21% 0.1086 199.72)",
	teal: "oklch(80% 0.128 168)",
	blue: "oklch(79.29% 0.1636 255.6)",

	// gray is reserved for core/foundation services
	gray: "oklch(69.4% 0 0)",
};

const toCSS = (a: Record<string, string>) =>
	Object.entries(a).map(([k, v]) => `${k}: ${v}`).join(";");

const badgeStyle = (bg: string, fg = "black") =>
	toCSS({
		color: fg,
		"background-color": bg,
		padding: "0 4px",
		"border-radius": "3px",
		"font-family": '"Comic Sans MS", "Comic Sans", cursive',
		display: "inline-block",
	});

const color = (fg: string) => toCSS({ color: fg });

type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

type LogEntry = {
	level: LogLevel;
	namespace: string;
	tag?: string;
	message: string;
	data: unknown;
};

const entries: Array<LogEntry> = [];

type LoggerNamespaceConfig = {
	color: string;
};

const namespaceConfig = new Map<string, LoggerNamespaceConfig>();

export const logger = {
	for(namespace: string) {
		const create = (level: LogLevel, customColor?: string) =>
		// log.info(message, data)
		// log.info(tag, message, data)
		(a: string, b?: unknown, c?: unknown) => {
			const tag = c ? a : undefined;
			const message = c ? b as string : a;
			const data = c ?? b;

			// entries.push({ level, namespace, tag, message, data });
			const cfg = namespaceConfig.get(namespace);
			const resolvedColor = customColor ?? cfg?.color ?? "white";

			const log: Array<unknown> = tag
				? [
					"%c%s%c %s%c %s",
					badgeStyle(resolvedColor),
					namespace,
					color(resolvedColor),
					tag,
				]
				: [
					"%c%s%c %s",
					badgeStyle(resolvedColor),
					namespace,
				];
			log.push("color:initial", message);
			if (data) log.push(data);
			console[level](...log);
		};

		return {
			error: create("error"),
			info: create("info"),
			warn: create("warn"),
			debug: create("debug"),
			trace: create("trace"),
			create,
		};
	},
	config(namespace: string, config: LoggerNamespaceConfig) {
		namespaceConfig.set(namespace, config);
	},
};

logger.config("sw", { color: colors.gray });
logger.config("config", { color: colors.gray });
logger.config("voice", { color: colors.green });
logger.config("rtc", { color: colors.cyan }); // webrtc
logger.config("vad", { color: colors.teal }); // voice activity detection
logger.config("cs", { color: colors.gray }); // client state
logger.config("api/dms", { color: colors.orange });
logger.config("api/threads", { color: colors.orange });
logger.config("api/emoji", { color: colors.orange });
logger.config("api/inbox", { color: colors.orange });
logger.config("api/push", { color: colors.orange });
logger.config("api/room_bans", { color: colors.orange });
logger.config("api/messages", { color: colors.orange });
logger.config("api/webhooks", { color: colors.orange });
logger.config("api/roles", { color: colors.orange });
logger.config("api/invite", { color: colors.orange });
logger.config("api/audit_log", { color: colors.orange });
logger.config("api/rooms", { color: colors.orange });
logger.config("idb", { color: colors.yellow });
logger.config("timeline", { color: colors.red });
// logger.config("user_popout", { color: colors.red });
// logger.config("permissions", { color: colors.green });
