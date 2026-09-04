use std::collections::HashMap;

use yrs::{Doc, ReadTxn, Transact};

use crate::{
    prelude::*,
    services::documents::{
        serialized::{FromDoc, ToDoc},
        util::DOCUMENT_ROOT_NAME,
    },
};

pub struct SerializedRedex {
    pub content: String,
}

impl FromDoc for SerializedRedex {
    type Error = Error;

    fn from_doc(doc: &Doc) -> Result<Self> {
        let txn = doc.transact();

        for (name, out) in txn.root_refs() {
            match (name, out) {
                (DOCUMENT_ROOT_NAME, yrs::Out::YText(text)) => {
                    // TODO: validate text?
                }
                ("metadata", yrs::Out::Any(yrs::Any::Map(meta))) => {
                    // TODO: parse metadata

                    // NOTE: i probably want to be able to access metadata outside of the Doc, which would require some code to sync metadata
                    // maybe only sync doc -> server and not the other way around?
                    // TODO: FromDoc/ToDoc for serde?
                    struct SerializedRedexMetadata {
                        // TODO: see crate-common/src/v2/types/redex.rs SerializedRedex, SerializedRedexFile
                        // r#type: RedexFileType,
                        name: String,

                        // TODO: validate that String is a valid path and Uuid is a valid subdocument
                        files: HashMap<String, Uuid>,
                    }

                    for (_k, _v) in meta.iter() {
                        todo!()
                    }
                }
                _ => {
                    return Err(Error::BadStatic("invalid root ref name or root ref type"));
                }
            }
        }

        todo!()
    }

    fn from_doc_lenient(doc: &Doc) -> Self {
        todo!()
    }
}

impl ToDoc for SerializedRedex {
    type Out = yrs::XmlFragmentPrelim;

    fn to_doc(&self) -> Self::Out {
        todo!()
    }
}
