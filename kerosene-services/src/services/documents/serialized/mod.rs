use std::collections::HashMap;

use common::v1::types::components::{Component, ComponentCanonical, ComponentId, ComponentType};
use common::v1::types::document::serialized::Serdoc;
use common::v1::types::error::ErrorField;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use yrs::types::xml::{XmlElementPrelim, XmlIn};
use yrs::{Doc, GetString, ReadTxn, Transact, XmlFragment, XmlOut, XmlTextPrelim};

use crate::prelude::*;
use crate::services::documents::util::DOCUMENT_ROOT_NAME;

// PERF: incremental document validation, use observe, only revalidate what changed
// TODO: better errors

// TODO: impl from for ApiError
#[derive(Debug, Default)]
pub struct DocumentParseError {
    pub fields: Vec<ErrorField>,
}

// TODO: use From/TryFrom instead of FromDoc, Into instead of ToDoc?

/// trait to deserialize a type from a yrs document
pub trait FromDoc: Sized {
    type Error;

    /// deserialize this type from a document
    ///
    /// strict validation, for loading docs from users
    fn from_doc(doc: &Doc) -> CoreResult<Self, Self::Error>;

    /// deserialize this type from a document
    ///
    /// lenient validation, for loading docs from the database
    fn from_doc_lenient(doc: &Doc) -> Self;
}

/// trait to serialize a type into a yrs document
// TODO: design this type better?
pub trait ToDoc: Sized {
    type Out: yrs::block::Prelim;
    fn to_doc(&self) -> Self::Out;
}

#[deprecated]
pub fn doc_to_serdoc(doc: &Doc) -> Serdoc {
    let s = SerializedProse::from_doc_lenient(doc);
    Serdoc {
        components: s.components,
    }
}

#[deprecated]
// TODO: move into ToDoc
pub fn serdoc_apply_to_doc(doc: &Doc, components: &[ComponentCanonical]) {
    let mut txn = doc.transact_mut();
    let root = doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);

    // clear existing data
    let len = root.len(&txn);
    if len > 0 {
        root.remove_range(&mut txn, 0, len);
    }

    for component in components {
        match &component.ty {
            ComponentType::Text { content } => {
                let root_len = root.len(&txn);
                let content: XmlIn = XmlTextPrelim::new(content).into();
                root.insert(
                    &mut txn,
                    root_len,
                    XmlElementPrelim::new("paragraph", [content]),
                );
            }
            _ => {
                // other components will come later
            }
        }
    }
}

// impl yrs::block::Prelim for SerializedProse {
//     type Return = yrs::block::Unused;

//     fn into_content(
//         self,
//         txn: &mut yrs::TransactionMut,
//     ) -> (yrs::block::ItemContent, Option<Self>) {
//         (yrs::block::ItemContent::Embed(todo!()), None)
//     }

//     fn integrate(self, _txn: &mut yrs::TransactionMut, _inner_ref: yrs::branch::BranchPtr) {}
// }

// // TODO: design this type better? maybe this could be used instead of FromDoc/ToDoc?
// trait DocumentFormat {
//     // serdoc
//     type Serialized;
//
//     // a serialized update that can be applied
//     type Update;
//
//     fn serialize(s: Self::Serialized, doc: &mut ());
//     fn deserialize(doc: ()) -> Self::Serialized;
//     fn validate_update(doc: (), update: ());
//     fn validate_doc(doc: ());
// }

// mod serde_doc; // TODO: remove this
mod prose;
mod redex;

pub use prose::SerializedProse;
pub use redex::SerializedRedex;
