use crate::ast::impl_ast;
use crate::prelude::*;
use lamprey_common::v2::types::{ChannelId, RoleId, UserId};

// PERF: stop calling .to_string() for everything

// formatting
pub struct Strong(SyntaxNode);
pub struct Emphasis(SyntaxNode);
pub struct Link(SyntaxNode);
pub struct Spoiler(SyntaxNode);
pub struct Code(SyntaxNode);

// terminal
pub struct Text(SyntaxNode);
pub struct Mention(SyntaxNode);
pub struct CustomEmoji(SyntaxNode);
pub struct UnicodeEmoji(SyntaxNode);

// maybe use this instead?
// pub struct Text(SyntaxToken);

#[derive(Debug, Clone)]
pub struct CustomEmojiData {
    pub animated: bool,
    pub name: String,
    pub id: Uuid,
}

/// the kind of a mention
pub enum MentionData {
    User(UserId),
    Role(RoleId),
    Channel(ChannelId),
    Everyone,
}

/// any inline node
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

impl AstNode for Inline {
    fn can_cast(tn: &SyntaxNode) -> bool {
        tn.kind().is_inline()
    }

    fn cast(tn: SyntaxNode) -> Result<Self, SyntaxNode> {
        if Strong::can_cast(&tn) {
            Ok(Self::Strong(Strong(tn)))
        } else if Emphasis::can_cast(&tn) {
            Ok(Self::Emphasis(Emphasis(tn)))
        } else if Link::can_cast(&tn) {
            Ok(Self::Link(Link(tn)))
        } else if Text::can_cast(&tn) {
            Ok(Self::Text(Text(tn)))
        } else if Mention::can_cast(&tn) {
            Ok(Self::Mention(Mention(tn)))
        } else if CustomEmoji::can_cast(&tn) {
            Ok(Self::CustomEmoji(CustomEmoji(tn)))
        } else if UnicodeEmoji::can_cast(&tn) {
            Ok(Self::UnicodeEmoji(UnicodeEmoji(tn)))
        } else if Spoiler::can_cast(&tn) {
            Ok(Self::Spoiler(Spoiler(tn)))
        } else if Code::can_cast(&tn) {
            Ok(Self::Code(Code(tn)))
        } else {
            Err(tn)
        }
    }

    fn node(&self) -> &SyntaxNode {
        match self {
            Inline::Strong(s) => s.node(),
            Inline::Emphasis(e) => e.node(),
            Inline::Link(l) => l.node(),
            Inline::Text(t) => t.node(),
            Inline::Mention(m) => m.node(),
            Inline::CustomEmoji(e) => e.node(),
            Inline::UnicodeEmoji(e) => e.node(),
            Inline::Spoiler(s) => s.node(),
            Inline::Code(c) => c.node(),
        }
    }
}

impl_ast!(Strong, NodeKind::Inline(InlineKind::Strong));
impl_ast!(Emphasis, NodeKind::Inline(InlineKind::Emphasis));
impl_ast!(Link, NodeKind::Inline(InlineKind::Link));
impl_ast!(Text, NodeKind::Text(TextKind::Text));
impl_ast!(Mention, NodeKind::Text(TextKind::Mention));
impl_ast!(CustomEmoji, NodeKind::Text(TextKind::CustomEmoji));
impl_ast!(UnicodeEmoji, NodeKind::Text(TextKind::UnicodeEmoji));
impl_ast!(Spoiler, NodeKind::Inline(InlineKind::Spoiler));
impl_ast!(Code, NodeKind::Inline(InlineKind::Code));

impl Strong {
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children()
            .filter_map(|child| Inline::cast(child).ok())
    }
}

impl Emphasis {
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children()
            .filter_map(|child| Inline::cast(child).ok())
    }
}

impl Link {
    /// get what this link is linking to
    pub fn href(&self) -> String {
        self.0
            .children()
            .find(|c| matches!(c.kind(), NodeKind::Text(TextKind::Url)))
            .map(|c| c.text().to_string())
            .expect("invalid link")
    }

    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children()
            .filter_map(|child| Inline::cast(child).ok())
    }

    pub fn is_automatic(&self) -> bool {
        todo!()
    }
}

impl Spoiler {
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children()
            .filter_map(|child| Inline::cast(child).ok())
    }
}

impl Code {
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children()
            .filter_map(|child| Inline::cast(child).ok())
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
