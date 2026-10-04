// TODO: plan how this will be implemented

// /** streaming html parser */
// declare module "lamprey:html" {
//   // for url unfurler
//   // copy stuff from html5ever and that old unfurl test thing i had
// }

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

// export class HtmlParser {
//   constructor(on_token: (token: HtmlToken) => void);
//   handle(response: JsResponse);
//   end(): void;
// }

// export type HtmlToken = { /* ... TODO ... */ }

// i could implement a stripped down DOMParser, but thats a can of worms. also, streaming html parsing is more memory efficient
