use common::{
    v1::types::components::{Component, ComponentCanonical, ComponentType},
    v2::types::components::ComponentId,
};
use yrs::{Doc, GetString, ReadTxn, Transact, XmlFragment};

use crate::{
    prelude::*,
    services::documents::{
        serialized::{FromDoc, ToDoc},
        util::DOCUMENT_ROOT_NAME,
    },
};

pub struct SerializedProse {
    pub components: Vec<ComponentCanonical>,
}

impl FromDoc for SerializedProse {
    type Error = Error;

    fn from_doc(doc: &Doc) -> Result<Self> {
        // TODO: Implement actual strict validation of the structure,
        // mirroring the logic previously in `validate.rs`

        let txn = doc.transact();
        let mut components = Vec::new();
        let mut next_id = 0;

        for (name, out) in txn.root_refs() {
            if name != DOCUMENT_ROOT_NAME {
                return Err(Error::BadStatic("invalid root ref name"));
            }

            // TODO: move into fn validate_node()?
            match out {
                yrs::Out::YXmlFragment(frag) => {
                    for child in frag.children(&txn) {
                        // TODO: move into fn validate_element()?
                        // let attrs: Vec<_> = e.attributes(tx).collect();
                        // for attr in e.attributes(tx) { }
                        match child {
                            yrs::XmlOut::Element(el) => {
                                // TODO: move into fn validate_element()
                                // let attrs: Vec<_> = e.attributes(tx).collect();
                                // for attr in e.attributes(tx) { }

                                match &**el.tag() {
                                    "root" => {
                                        // no attrs
                                        // can only contain markdown elements
                                    }
                                    "markdown" | "text" | "paragraph" => {
                                        // standardize on one tag name, deprecate the rest?
                                        // no attrs
                                        // can only contain text

                                        // TODO: verify that this works correctly
                                        let content = el.get_string(&txn);
                                        components.push(Component {
                                            id: ComponentId(next_id),
                                            ty: ComponentType::Text { content },
                                            allow: None,
                                        });
                                        next_id += 1;
                                    }
                                    "media" => {
                                        // copy components validation (min 1 max 20 items, etc)
                                        // no content
                                    }
                                    // TODO: container, section, details
                                    _ => {
                                        return Err(Error::BadStatic("unknown node type"));
                                    }
                                }
                            }
                            yrs::XmlOut::Fragment(f) => {
                                // TODO: validate_fragment
                                // call validate_node for each part
                            }
                            yrs::XmlOut::Text(_) => {
                                // text is always allowed(?)
                            }
                        }
                    }
                }

                _ => return Err(Error::BadStatic("invalid yrs type for `content`")),
            }
        }

        Ok(Self { components })
    }

    fn from_doc_lenient(doc: &Doc) -> Result<Self> {
        todo!()
    }
}

impl ToDoc for SerializedProse {
    type Out = yrs::XmlFragmentPrelim;

    fn to_doc(&self) -> Self::Out {
        todo!()
    }
}
