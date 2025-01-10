import { OpenAPIHono } from "@hono/zod-openapi";
import { HonoEnv } from "globals";
import { apiReference } from "npm:@scalar/hono-api-reference";

export default function setup(app: OpenAPIHono<HonoEnv>) {
  app.doc("/api/docs.json", {
  	openapi: "3.0.0",
  	info: {
  		version: "0.0.1",
  		title: "My API",
  		description: "work in progress docs",
  	},
  	// security: [{
  	//   type: "apiKey",
  	//   name: "authorization",
  	//   in: "header",
  	// }],
  	tags: [
  		{
  			name: "room",
  			description:
  				"routes for messing with rooms. roughly similar to a discord guild or group dm, or a matrix room.",
  		},
  		{
  			name: "thread",
  			description:
  				"can be cheaply created and archived, but kept as long as needed. somewhat between discord channels and threads.",
  		},
  		{
  			name: "sessions",
  			description:
  				"currently not very useful since there's no way to authenticate a session",
  		},
  		{
  			name: "auth",
  			description:
  				"discord auth is the only supported method for now, will add other auth methods later",
  		},
  		{
  			name: "media",
  			description:
  				"work in progress api. will probably come up with something less terrible later.",
  		},
  	],
  	servers: [
  		{ url: "https://chat.celery.eu.org", description: "production" },
  		{ url: "http://localhost:8000", description: "local dev" },
  	],
  });

  app.get(
  	"/api/docs/*",
  	apiReference({
  		theme: "saturn",
  		pageTitle: "api reference",
  		pathRouting: { basePath: "/api/docs" },
  		layout: "modern",
  		// withDefaultFonts: false,
  		hideClientButton: true,
  		spec: { url: "/api/docs.json" },
  	}),
  );
}
