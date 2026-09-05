use common::{
    v1::types::components::{Component, ComponentCanonical, ComponentType},
    v2::types::components::ComponentId,
};
use yrs::{
    Doc, GetString, Out, ReadTxn, Transact, Transaction, Xml, XmlElementRef, XmlFragment, XmlOut,
};

use crate::{
    prelude::*,
    services::documents::{
        serialized::{FromDoc, ToDoc},
        util::DOCUMENT_ROOT_NAME,
    },
};
use thiserror::Error;

pub struct SerializedProse {
    pub components: Vec<ComponentCanonical>,
}

// TODO: rework FromDoc/ToDoc to use this directly(?)
#[derive(Debug, Default)]
struct SerializedProse2 {
    components: Vec<ComponentCanonical>,
    errors: Vec<SerializedProseError>,
    next_id: u16,
}

#[derive(Debug, Error)]
pub enum SerializedProseError {
    #[error("invalid root ref name")]
    InvalidRootName,
    #[error("unknown node type")]
    UnknownNodeType,
    #[error("invalid yrs type for `content`")]
    InvalidYrsType,
    #[error("invalid inline node")]
    InvalidInlineNode,
    #[error("missing attribute")]
    MissingAttribute,
    #[error("invalid attribute type")]
    InvalidAttributeType,
}

impl SerializedProse2 {
    fn from_doc(doc: &Doc) -> Self {
        // TODO: Implement actual strict validation of the structure,
        // mirroring the logic previously in `validate.rs`

        let txn = doc.transact();
        let mut me = Self::default();

        for (name, out) in txn.root_refs() {
            if name == DOCUMENT_ROOT_NAME {
                match out.cast() {
                    Ok(out) => me.handle_node(&txn, &out),
                    Err(_) => me.errors.push(SerializedProseError::InvalidYrsType),
                }
            } else {
                me.errors.push(SerializedProseError::InvalidRootName);
            }
        }

        me
    }

    fn handle_node(&mut self, txn: &Transaction<'_>, node: &XmlOut) {
        // TODO: move into fn validate_element()?
        // let attrs: Vec<_> = e.attributes(tx).collect();
        // for attr in e.attributes(tx) { }
        match node {
            XmlOut::Element(el) => match &**el.tag() {
                "root" => {
                    // TODO(?): enforce that only certain nodes can be in root
                    // TODO(?): enforce that all documents contain exactly one root
                    // TODO: enforce that root elements cannot contain other roots (or that root elements must appear at the top level)
                    for child in el.children(txn) {
                        self.handle_node(txn, &child);
                    }
                }

                // TODO: standardize on one tag name and deprecate the rest
                "markdown" | "text" | "paragraph" => {
                    let content = self.xml_to_markdown(txn, el);
                    self.components.push(Component {
                        id: ComponentId(self.next_id),
                        ty: ComponentType::Text { content },
                        allow: None,
                    });
                    self.next_id += 1;
                }

                "media" => {
                    // TODO: copy components validation (min 1 max 20 items, etc)
                    // this element should no content
                }

                // TODO: handle container, section, details
                _ => self.errors.push(SerializedProseError::UnknownNodeType),
            },
            XmlOut::Text(_) => {
                // text is allowed here?
            }
            XmlOut::Fragment(_) => {
                unreachable!("fragments cant contain other fragments")
            }
        }
    }

    /// convert a yrs xml node's content into markdown
    fn xml_to_markdown(&mut self, txn: &Transaction<'_>, el: &XmlElementRef) -> String {
        let mut out = String::new();

        for child in el.children(txn) {
            match child {
                XmlOut::Text(t) => {
                    out.push_str(&t.get_string(txn));
                }
                XmlOut::Element(e) => match &**e.tag() {
                    "mention" => match e.get_attribute(txn, "user") {
                        Some(user) => match out_as_str(&user) {
                            Ok(u) => out.push_str(&format!("<@{}>", u)),
                            Err(e) => self.errors.push(e),
                        },
                        None => self.errors.push(SerializedProseError::MissingAttribute),
                    },
                    "mentionChannel" => match e.get_attribute(txn, "channel") {
                        Some(channel) => match out_as_str(&channel) {
                            Ok(c) => out.push_str(&format!("<#{}>", c)),
                            Err(e) => self.errors.push(e),
                        },
                        None => self.errors.push(SerializedProseError::MissingAttribute),
                    },
                    "mentionRole" => match e.get_attribute(txn, "role") {
                        Some(role) => match out_as_str(&role) {
                            Ok(r) => out.push_str(&format!("<@&{}>", r)),
                            Err(e) => self.errors.push(e),
                        },
                        None => self.errors.push(SerializedProseError::MissingAttribute),
                    },
                    "mentionEveryone" => {
                        out.push_str("@everyone");
                    }
                    "emojiCustom" => match e.get_attribute(txn, "name") {
                        Some(name) => match out_as_str(&name) {
                            Ok(n) => out.push_str(&format!(":{}:", n)),
                            Err(e) => self.errors.push(e),
                        },
                        None => self.errors.push(SerializedProseError::MissingAttribute),
                    },
                    "emojiUnicode" => match e.get_attribute(txn, "char") {
                        Some(char) => match out_as_str(&char) {
                            Ok(c) => out.push_str(c),
                            Err(e) => self.errors.push(e),
                        },
                        None => self.errors.push(SerializedProseError::MissingAttribute),
                    },
                    _ => self.errors.push(SerializedProseError::InvalidInlineNode),
                },
                XmlOut::Fragment(_) => unreachable!("fragments cant contain other fragments"),
            }
        }

        out
    }
}

impl FromDoc for SerializedProse {
    type Error = SerializedProseError;

    fn from_doc(doc: &Doc) -> CoreResult<Self, Self::Error> {
        let p = SerializedProse2::from_doc(doc);
        if p.errors.is_empty() {
            Ok(Self {
                components: p.components,
            })
        } else {
            Err(todo!())
        }
    }

    fn from_doc_lenient(doc: &Doc) -> Self {
        Self {
            components: SerializedProse2::from_doc(doc).components,
        }
    }
}

impl ToDoc for SerializedProse {
    type Out = yrs::XmlFragmentPrelim;

    fn to_doc(&self) -> Self::Out {
        todo!()
    }
}

fn out_as_str(a: &Out) -> CoreResult<&str, SerializedProseError> {
    match a {
        Out::Any(yrs::Any::String(s)) => Ok(&*s),
        _ => Err(SerializedProseError::InvalidAttributeType),
    }
}
