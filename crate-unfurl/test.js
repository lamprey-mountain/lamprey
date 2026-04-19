/// <reference path="unfurl.d.ts"/>

import { HtmlParser } from "lamprey:html";

export const name = "test_plugin";

new TextDecoder();
// new Url();

export async function processResponse(url, res) {
    let title = "";

    const parser = new HtmlParser((token) => {
        if (token.type === "StartTag" && token.name === "title") {
           // logic
        }
    });

    await parser.feed_response(res);
    return [{ title, url }];
}

// export const name = "generic_scraper";

// export async function processResponse(url, res) {
//     let extracted = { title: "", description: "" };
//     let inTitle = false;

//     const parser = new HtmlParser((token) => {
//         if (token.type === "StartTag" && token.name === "title") {
//             inTitle = true;
//         } else if (token.type === "EndTag" && token.name === "title") {
//             inTitle = false;
//         } else if (token.type === "Text" && inTitle) {
//             extracted.title += token.content;
//         }
//     });

//     // This consumes the response stream
//     await parser.handle(res);

//     return [{
//         title: extracted.title.trim(),
//         url: url
//     }];
// }
