use crate::prelude::*;
use crate::{ast::impl_ast, tree::node::SyntaxElement};
use lamprey_common::v2::types::{ChannelId, RoleId, UserId};

// PERF: stop calling .to_string() for everything

// formatting
#[derive(Debug)]
pub struct Strong(SyntaxNode);
#[derive(Debug)]
pub struct Emphasis(SyntaxNode);
#[derive(Debug)]
pub struct Link(SyntaxNode);
#[derive(Debug)]
pub struct Spoiler(SyntaxNode);
#[derive(Debug)]
pub struct Code(SyntaxNode);

// terminal
#[derive(Debug)]
pub struct Text(SyntaxToken);
#[derive(Debug)]
pub struct Mention(SyntaxToken);
#[derive(Debug)]
pub struct CustomEmoji(SyntaxToken);
#[derive(Debug)]
pub struct UnicodeEmoji(SyntaxToken);

#[derive(Debug, Clone)]
pub struct CustomEmojiData {
    pub animated: bool,
    pub name: String,
    pub id: Uuid,
}

/// the kind of a mention
#[derive(Debug)]
pub enum MentionData {
    User(UserId),
    Role(RoleId),
    Channel(ChannelId),
    Everyone,
}

/// any inline node
#[derive(Debug)]
pub enum Inline {
    Strong(Strong),
    Emphasis(Emphasis),
    Link(Link),
    Spoiler(Spoiler),
    Code(Code),

    Text(Text),
    Mention(Mention),
    CustomEmoji(CustomEmoji),
    UnicodeEmoji(UnicodeEmoji),
}

impl Inline {
    pub fn cast(el: SyntaxElement) -> Option<Self> {
        match el {
            SyntaxElement::Node(node) => {
                let kind = node.kind();
                if Strong::can_cast(kind) {
                    Strong::cast(node).map(Self::Strong)
                } else if Emphasis::can_cast(kind) {
                    Emphasis::cast(node).map(Self::Emphasis)
                } else if Link::can_cast(kind) {
                    Link::cast(node).map(Self::Link)
                } else if Spoiler::can_cast(kind) {
                    Spoiler::cast(node).map(Self::Spoiler)
                } else if Code::can_cast(kind) {
                    Code::cast(node).map(Self::Code)
                } else {
                    None
                }
            }
            SyntaxElement::Token(token) => {
                let kind = token.kind();
                if Text::can_cast(kind) {
                    Text::cast(token).map(Self::Text)
                } else if Mention::can_cast(kind) {
                    Mention::cast(token).map(Self::Mention)
                } else if CustomEmoji::can_cast(kind) {
                    CustomEmoji::cast(token).map(Self::CustomEmoji)
                } else if UnicodeEmoji::can_cast(kind) {
                    UnicodeEmoji::cast(token).map(Self::UnicodeEmoji)
                } else {
                    None
                }
            }
        }
    }

    pub fn syntax(&self) -> SyntaxElement {
        match self {
            Inline::Strong(s) => SyntaxElement::Node(s.syntax().clone()),
            Inline::Emphasis(e) => SyntaxElement::Node(e.syntax().clone()),
            Inline::Link(l) => SyntaxElement::Node(l.syntax().clone()),
            Inline::Spoiler(s) => SyntaxElement::Node(s.syntax().clone()),
            Inline::Code(c) => SyntaxElement::Node(c.syntax().clone()),
            Inline::Text(t) => SyntaxElement::Token(t.syntax().clone()),
            Inline::Mention(m) => SyntaxElement::Token(m.syntax().clone()),
            Inline::CustomEmoji(e) => SyntaxElement::Token(e.syntax().clone()),
            Inline::UnicodeEmoji(e) => SyntaxElement::Token(e.syntax().clone()),
        }
    }
}

impl_ast!(Strong, NodeKind::Inline(InlineKind::Strong));
impl_ast!(Emphasis, NodeKind::Inline(InlineKind::Emphasis));
impl_ast!(Link, NodeKind::Inline(InlineKind::Link));
impl_ast!(Spoiler, NodeKind::Inline(InlineKind::Spoiler));
impl_ast!(Code, NodeKind::Inline(InlineKind::Code));

// TODO: consider creating a trait for this? like AstToken or something?
// or maybe get rid of special handling for tokens/TextKind and make everything use nodes
macro_rules! impl_token {
    ($name:ident, $kind:pat $(if $guard:expr)?) => {
        impl $name {
            pub fn can_cast(kind: NodeKind) -> bool {
                matches!(kind, $kind $(if $guard)?)
            }

            pub fn cast(token: SyntaxToken) -> Option<Self> {
                if Self::can_cast(token.kind()) {
                    Some(Self(token))
                } else {
                    None
                }
            }

            pub fn syntax(&self) -> &SyntaxToken {
                &self.0
            }
        }
    };
}

impl_token!(Text, NodeKind::Text(TextKind::Text));
impl_token!(Mention, NodeKind::Text(TextKind::Mention));
impl_token!(CustomEmoji, NodeKind::Text(TextKind::CustomEmoji));
impl_token!(UnicodeEmoji, NodeKind::Text(TextKind::UnicodeEmoji));

impl Strong {
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| Inline::cast(child))
    }
}

impl Emphasis {
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| Inline::cast(child))
    }
}

impl Link {
    /// get what this link is linking to
    pub fn href(&self) -> String {
        self.0
            .children_with_tokens()
            .find(|c| matches!(c.kind(), NodeKind::Text(TextKind::Url)))
            .map(|c| c.to_string())
            .expect("invalid link")
    }

    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| Inline::cast(child))
    }

    pub fn is_automatic(&self) -> bool {
        todo!()
    }
}

impl Spoiler {
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| Inline::cast(child))
    }
}

impl Code {
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| Inline::cast(child))
    }
}

// terminal nodes

impl Text {
    /// get the text content of this ast
    pub fn text(&self) -> String {
        self.0.text().to_string()
    }
}

impl UnicodeEmoji {
    /// get the text content of this ast
    pub fn text(&self) -> String {
        self.0.text().to_string()
    }
}

impl Mention {
    /// get the serialized text content of this mention
    pub fn text(&self) -> String {
        self.0.text().to_string()
    }

    pub fn parse(&self) -> MentionData {
        let text = self.0.text().to_string();
        if text.starts_with("<@") && text.ends_with('>') {
            let uuid: Uuid = text[2..text.len() - 1]
                .parse()
                .expect("invalid Mention content");
            MentionData::User(UserId::from(uuid))
        } else if text.starts_with("<&") && text.ends_with('>') {
            let uuid: Uuid = text[2..text.len() - 1]
                .parse()
                .expect("invalid Mention content");
            MentionData::Role(RoleId::from(uuid))
        } else if text.starts_with("<#") && text.ends_with('>') {
            let uuid: Uuid = text[2..text.len() - 1]
                .parse()
                .expect("invalid Mention content");
            MentionData::Channel(ChannelId::from(uuid))
        } else {
            MentionData::Everyone
        }
    }
}

impl CustomEmoji {
    /// get the serialized text content of this custom emoji
    pub fn text(&self) -> String {
        self.0.text().to_string()
    }

    pub fn parse(&self) -> CustomEmojiData {
        let text = self.0.text().to_string();
        let is_animated = text.starts_with("<a:");
        let parts: Vec<&str> = text[1..text.len() - 1].split(':').collect();
        let (name, id_str) = if is_animated {
            (parts[1], parts[2])
        } else {
            (parts[0], parts[1])
        };
        CustomEmojiData {
            animated: is_animated,
            name: name.to_string(),
            id: Uuid::parse_str(id_str).unwrap_or_default(),
        }
    }
}
