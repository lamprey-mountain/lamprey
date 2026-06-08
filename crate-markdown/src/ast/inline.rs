use crate::ast::impl_ast;
use crate::prelude::*;
use lamprey_common::v2::types::{ChannelId, RoleId, UserId};

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

#[derive(Debug, Clone)]
pub struct CustomEmojiData {
    pub animated: bool,
    // PERF: consider using &'a str
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
    Text(Text),
    Mention(Mention),
    CustomEmoji(CustomEmoji),
    UnicodeEmoji(UnicodeEmoji),
    Spoiler(Spoiler),
    Code(Code),
}

impl AstNode for Inline {
    fn can_cast(node: &SyntaxData) -> bool {
        node.kind().is_inline() || matches!(node.kind(), NodeKind::Text(_))
    }

    fn cast(tn: SyntaxNode) -> Result<Self, SyntaxNode> {
        if Strong::can_cast(&tn.node) {
            Ok(Self::Strong(Strong(tn)))
        } else if Emphasis::can_cast(&tn.node) {
            Ok(Self::Emphasis(Emphasis(tn)))
        } else if Link::can_cast(&tn.node) {
            Ok(Self::Link(Link(tn)))
        } else if Text::can_cast(&tn.node) {
            Ok(Self::Text(Text(tn)))
        } else if Mention::can_cast(&tn.node) {
            Ok(Self::Mention(Mention(tn)))
        } else if CustomEmoji::can_cast(&tn.node) {
            Ok(Self::CustomEmoji(CustomEmoji(tn)))
        } else if UnicodeEmoji::can_cast(&tn.node) {
            Ok(Self::UnicodeEmoji(UnicodeEmoji(tn)))
        } else if Spoiler::can_cast(&tn.node) {
            Ok(Self::Spoiler(Spoiler(tn)))
        } else if Code::can_cast(&tn.node) {
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
    pub fn href(&self) -> &str {
        self.0
            .children()
            .find(|c| matches!(c.node.kind(), NodeKind::Text(TextKind::Url)))
            .map(|c| {
                let span = c.node.span();
                &self.0.tree.source()[span.start as usize..span.end as usize]
            })
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
    pub fn text(&self) -> &str {
        self.0.text()
    }
}

impl UnicodeEmoji {
    /// get the text content of this ast
    pub fn text(&self) -> &str {
        self.0.text()
    }
}

impl Mention {
    /// get the serialized text content of this mention
    pub fn text(&self) -> &str {
        self.0.text()
    }

    pub fn parse(&self) -> MentionData {
        let text = self.0.text();
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
    pub fn text(&self) -> &str {
        self.0.text()
    }

    pub fn parse(&self) -> CustomEmojiData {
        let text = self.0.text();
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
