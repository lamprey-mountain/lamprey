use lamprey_common::v2::types::{ChannelId, RoleId, UserId};

use crate::prelude::*;
use crate::parser::Parsed;

// #[cfg_attr(feature = "wasm", wasm_bindgen)]
// TODO: concrete types instead of impl Iterator
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
