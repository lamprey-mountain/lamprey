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

impl FromDoc for SerializedProse {
    type Error = SerializedProseError;

    fn from_doc(doc: &Doc) -> CoreResult<Self, Self::Error> {
        // TODO: Implement actual strict validation of the structure,
        // mirroring the logic previously in `validate.rs`

        let txn = doc.transact();
        let mut components = Vec::new();
        let mut next_id = 0;

        for (name, out) in txn.root_refs() {
            if name != DOCUMENT_ROOT_NAME {
                return Err(SerializedProseError::InvalidRootName);
            }

            let out: XmlOut = out.cast().unwrap();
            handle_node(&txn, &out, &mut components, &mut next_id)?;
        }

        Ok(Self { components })
    }

    fn from_doc_lenient(doc: &Doc) -> Self {
        todo!()
    }
}

impl ToDoc for SerializedProse {
    type Out = yrs::XmlFragmentPrelim;

    fn to_doc(&self) -> Self::Out {
        todo!()
    }
}

fn handle_node(
    txn: &Transaction<'_>,
    node: &XmlOut,
    components: &mut Vec<ComponentCanonical>,
    next_id: &mut u16,
) -> CoreResult<(), SerializedProseError> {
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
                    handle_node(txn, &child, components, next_id)?;
                }
            }

            // TODO: standardize on one tag name and deprecate the rest
            "markdown" | "text" | "paragraph" => {
                let content = xml_to_markdown(txn, el)?;
                components.push(Component {
                    id: ComponentId(*next_id),
                    ty: ComponentType::Text { content },
                    allow: None,
                });
                *next_id += 1;
            }

            "media" => {
                // TODO: copy components validation (min 1 max 20 items, etc)
                // this element should no content
            }

            // TODO: handle container, section, details
            _ => return Err(SerializedProseError::UnknownNodeType),
        },
        XmlOut::Text(_) => {
            // text is allowed here?
        }
        XmlOut::Fragment(_) => {
            unreachable!("fragments cant contain other fragments")
        }
    }

    Ok(())
}

/// convert a yrs xml node's content into markdown
fn xml_to_markdown(
    txn: &Transaction<'_>,
    el: &XmlElementRef,
) -> CoreResult<String, SerializedProseError> {
    let mut out = String::new();
    for child in el.children(txn) {
        match child {
            XmlOut::Text(t) => {
                out.push_str(&t.get_string(txn));
            }
            XmlOut::Element(e) => match &**e.tag() {
                "mention" => {
                    let user = e
                        .get_attribute(txn, "user")
                        .ok_or(SerializedProseError::MissingAttribute)?;
                    out.push_str(&format!("<@{}>", out_as_str(&user)?));
                }
                "mentionChannel" => {
                    let channel = e
                        .get_attribute(txn, "channel")
                        .ok_or(SerializedProseError::MissingAttribute)?;
                    out.push_str(&format!("<#{}>", out_as_str(&channel)?));
                }
                "mentionRole" => {
                    let role = e
                        .get_attribute(txn, "role")
                        .ok_or(SerializedProseError::MissingAttribute)?;
                    out.push_str(&format!("<@&{}>", out_as_str(&role)?));
                }
                "mentionEveryone" => {
                    out.push_str("@everyone");
                }
                "emojiCustom" => {
                    let name = e
                        .get_attribute(txn, "name")
                        .ok_or(SerializedProseError::MissingAttribute)?;
                    out.push_str(&format!(":{}:", out_as_str(&name)?));
                }
                "emojiUnicode" => {
                    let char = e
                        .get_attribute(txn, "char")
                        .ok_or(SerializedProseError::MissingAttribute)?;
                    out.push_str(out_as_str(&char)?);
                }
                _ => return Err(SerializedProseError::InvalidInlineNode),
            },
            XmlOut::Fragment(_) => unreachable!("fragments cant contain other fragments"),
        }
    }
    Ok(out)
}

fn out_as_str(a: &Out) -> CoreResult<&str, SerializedProseError> {
    match a {
        Out::Any(yrs::Any::String(s)) => Ok(&*s),
        _ => Err(SerializedProseError::InvalidAttributeType),
    }
}
