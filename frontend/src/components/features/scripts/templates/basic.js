/// <reference path="./exports.d.ts"/>

import { Response } from "lamprey:http";

// this log line will run during extraction
log.info("hello, world!");

// log lines can have metadata
log.info("here's some metadata", {
	apple: "red",
	orange: "orange",
	banana: "yellow",
});

// logging can be done at different log levels
log.debug("setup log");

// you configure redex names and other metadata with a default export
export default {
	name: "My first script!",
	description: "This is an example script",
};

// register entrypoints for this redex
// this can be exported in the default export, but exporting a separate function works too
export function register(ctl) {
	// handle a trigger
	// triggers are the most basic entrypoint. they must be explicitly/manually called by the user or other applications.
	ctl
		.onTrigger()
		.id("foobar")
		.label("Do something")
		.run(() => {
			log.info("handle trigger!");
		});

	// handle http requests
	ctl
		.onHttp()
		.id("http")
		.run((req) => {
			log.info("handle http request!", { method: req.method, url: req.url });
			return new Response(`hello, world! method: ${req.method}`, {
				status: 200,
			});
		});
}
