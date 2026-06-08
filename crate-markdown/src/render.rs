use lamprey_common::v2::types::{ChannelId, RoleId, UserId};

use crate::parser::Parsed;
use crate::prelude::*;

// #[cfg_attr(feature = "wasm", wasm_bindgen)]
// TODO: concrete types instead of impl Iterator
// TODO: use Render trait currently in ast/mod.rs
impl Parsed {
    /// render to html
    pub fn to_html(&self) -> String {
        let mut cursor = self.cursor();
        todo!("traverse with cursor")
    }

    /// render to plaintext, stripping any formatting
    pub fn to_plain(&self) -> String {
        todo!()
    }

    /// render to markdown
    pub fn to_markdown(&self) -> String {
        todo!()
    }

    // fn strip_emoji(&mut self, allowed_emojis: ())

    /// iterate over all links
    pub fn iter_links(&self) -> impl Iterator<Item = Link> {
        // TODO
        vec![].into_iter()
    }

    /// iterate over all mentions
    pub fn iter_mentions(&self) -> impl Iterator<Item = Mention> {
        // TODO
        vec![].into_iter()
    }
}

// TODO: move these to ast module?
/// a link extracted from markdown
pub struct Link {
    // href, text
}

/// a mention extracted from markdown
pub struct Mention {
    pub kind: MentionKind,
    // pub text: String
    // pub text: &' str
    // pub text: cow?
    // pub text: Span
}

/// the kind of a mention
pub enum MentionKind {
    User(UserId),
    Role(RoleId),
    Channel(ChannelId),
    Everyone,
}

/// a markdown heading
pub struct Heading {
    // TODO
}

/// a decoration that can be applied to the markdown source
// TODO: better types
pub struct Decoration {
    pub span: Span,
    pub attrs: DecorationAttrs,
    // options?: { inclusiveStart?: boolean; inclusiveEnd?: boolean };
}

pub struct DecorationAttrs {
    // consider making strings &' static str
    pub node_name: String,
    pub class: String,
    pub style: String,
}

// pub struct DecorationAttr {}
// pub enum DecorationClass {
//     Syn,
//     SynCodeLang,
//     Em,
//     B,
//     Spoiler,
//     Link,
// }

// impl DecorationAttr {
//     /// get the class name for this node
//     pub fn class_name(&self) -> &str {
//         todo!()
//     }
// }

// pub enum DecorationEvent {
//     Add,
//     Remove,
// }
