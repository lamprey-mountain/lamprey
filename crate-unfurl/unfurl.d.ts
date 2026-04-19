/// <reference no-default-lib="true"/>

declare const self: Globals;

type Globals = {
  // processResponse: undefined | ((url: string, res: JsResponse) => unknown);
};

/** streaming html parser */
declare module "lamprey:html" {
  // // copy from html5ever
  // pub enum Token {
  //     /// A DOCTYPE declaration like `<!DOCTYPE html>`
  //     DoctypeToken(Doctype),
  //     /// A opening or closing tag, like `<foo>` or `</bar>`
  //     TagToken(Tag),
  //     /// A comment like `<!-- foo -->`.
  //     CommentToken(StrTendril),
  //     /// A sequence of characters.
  //     CharacterTokens(StrTendril),
  //     /// A `U+0000 NULL` character in the input.
  //     NullCharacterToken,
  //     EOFToken,
  //     ParseError(Cow<'static, str>),
  // }

  export class HtmlParser {
    constructor(on_token: (token: HtmlToken) => void);
    handle(response: JsResponse);
    end(): void;
  }

  export type HtmlToken = { /* ... TODO ... */ }
}

/** stream utilities */
declare module "lamprey:stream" {
  // TODO
}

interface JsEmbed {
  title?: string;
  description?: string;
  url?: string;
  siteName?: string;
  color?: string; // e.g. "#ff0000"
  image?: string;
}

interface JsResponse {
  body: string;
  status: number;
}

interface UnfurlPlugin {
  name: string;
  /** Return null if this plugin doesn't handle the URL */
  processUrl?(url: string): JsEmbed[] | null;
  /** Process the full HTML/Response */
  processResponse?(url: string, res: JsResponse): JsEmbed[];
}

/*

// try to make it as close to the web standard api as possible?
// see if quickjs has web api types built in? if not, maybe follow loosely so that i dont need to implement *everything*
type Response {
  status,
  headers,
  chunk(): Promise<Uint8Array>;
  bytes(): Uint8Array;
  text(): string;
}

class Logger {
  note(); // debug logging
  warn();
  error();
  fatal(); // like error, but also exits
}
*/

declare module "lamprey:unfurl" {
  // move all unfurler-specific stuff here?
}
