// TODO: validating documents

use std::ops::Deref;

use yrs::{Doc, Map, ReadTxn, Text, TextPrelim, Transact, WriteTxn, Xml, XmlFragment};

use crate::{Error, Result};

/// trait to deserialize a type from a yrs document
pub trait FromCrdt: Sized {
    type Error;

    // i need a strict mode for loading docs from users and a lenient mode for loading docs from the database
    fn deserialize(doc: &Doc) -> ::std::result::Result<Self, Self::Error>;
}

/// trait to serialize a type into a yrs document
pub trait IntoCrdt: Sized {
    // TODO: design this type

    type Out: yrs::block::Prelim;
    fn arst(self) -> Self::Out;
}

struct SerializedProse;
struct SerializedRedex;

// impl FromCrdt for SerializedProse {}
// impl FromCrdt for SerializedRedex {}

impl yrs::block::Prelim for SerializedProse {
    type Return = yrs::block::Unused;

    fn into_content(
        self,
        txn: &mut yrs::TransactionMut,
    ) -> (yrs::block::ItemContent, Option<Self>) {
        (yrs::block::ItemContent::Embed(todo!()), None)
    }

    fn integrate(self, _txn: &mut yrs::TransactionMut, _inner_ref: yrs::branch::BranchPtr) {}
}

/// validates this prose document
pub fn validate_prose(doc: &Doc) -> Result<()> {
    let tx = doc.transact();
    for (name, out) in tx.root_refs() {
        if name != "content" {
            return Err(Error::BadStatic("invalid root ref name"));
        }

        match out {
            yrs::Out::YXmlFragment(frag) => {
                for c in frag.children(&tx) {
                    // TODO: move into fn validate_node()
                    match c {
                        yrs::XmlOut::Element(e) => {
                            // TODO: move into fn validate_element()
                            // let attrs: Vec<_> = e.attributes(tx).collect();
                            // for attr in e.attributes(tx) { }

                            match e.tag().deref() {
                                "root" => {
                                    // no attrs
                                    // can only contain markdown elements
                                }
                                "markdown" | "text" | "paragraph" => {
                                    // standardize on one tag name, deprecate the rest?
                                    // no attrs
                                    // can only contain text
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

    Ok(())
}

/// validates this redex document
pub fn validate_redex(doc: &Doc) -> Result<()> {
    let doc: &mut Doc = todo!();
    let mut txn = doc.transact_mut();
    txn.get_or_insert_map("name")
        .insert(&mut txn, "./path/to/file", Doc::new());
    // let r = txn.get_or_insert_text("aaaaaaaa");
    // r.apply_delta(&mut txn, delta);
    // r.insert(txn, index, chunk);
    // r.insert_with_attributes(&mut txn, index, chunk, attributes);
    // r.insert_embed(&mut txn, index, content);
    // r.push(&mut txn, chunk);
    // r.format(&mut txn, index, len, attributes);

    let tx = doc.transact();
    // tx.subdocs().next().unwrap();
    for (name, out) in tx.root_refs() {
        match name {
            "files" => {
                let yrs::Out::YMap(map) = out else {
                    // TODO: validate filename (`name`)
                    return Err(Error::BadStatic("invalid yrs type for `files`"));
                };

                // TODO: validate that map is Map<ValidPath, Subdocument>
                // TODO: validate that ValidPath is a String that's a valid path
                // PERF: incremental validation, use observe, only revalidate what changed

                // redex subdocument validation:
                // - name `content`, type Out::YText - textual content of the script itself
                // - maybe allow Out::Any(Any::Buffer(_)) for webassembly?
                // - file types: text, media (media id), folder
            }
            "metadata" => {
                let yrs::Out::Any(any) = out else {
                    return Err(Error::BadStatic("invalid yrs type for `metadata`"));
                };

                // NOTE: i probably want to store metadata outside of the Doc, but could this still be useful when serializing?
            }
            _ => return Err(Error::BadStatic("invalid root ref name")),
        }
    }

    Ok(())
}

// /// validates this document
// pub fn validate(doc: &Doc) -> Result<()> {
//     // TODO: validate that no other fragments exist?
//     let xml = doc.get_or_insert_xml_fragment("content");
//     let tx = doc.transact();

//     for c in xml.children(&tx) {
//         // TODO: move into fn validate_node()
//         match c {
//             yrs::XmlOut::Element(e) => {
//                 // TODO: move into fn validate_element()
//                 // let attrs: Vec<_> = e.attributes(tx).collect();
//                 // for attr in e.attributes(tx) {

//                 // }
//                 // attrs[0].1.
//                 // match attrs[0].1 {
//                 //     yrs::Out::Any(any) => todo!(),
//                 //     yrs::Out::YText(text_ref) => todo!(),
//                 //     yrs::Out::YArray(array_ref) => todo!(),
//                 //     yrs::Out::YMap(map_ref) => todo!(),
//                 //     yrs::Out::YXmlElement(xml_element_ref) => todo!(),
//                 //     yrs::Out::YXmlFragment(xml_fragment_ref) => todo!(),
//                 //     yrs::Out::YXmlText(xml_text_ref) => todo!(),
//                 //     yrs::Out::YDoc(doc) => todo!(),
//                 //     yrs::Out::UndefinedRef(branch_ptr) => todo!(),
//                 // }

//                 // minimal format for now; only has root and markdown. embedded media and such will come later.
//                 match e.tag().deref() {
//                     "root" => {
//                         // no attrs
//                         // can only contain markdown elements
//                     }
//                     "markdown" => {
//                         // no attrs
//                         // can only contain text
//                     }
//                     _ => {
//                         return Err(Error::BadStatic("unknown node type"));
//                     }
//                 }
//             }
//             yrs::XmlOut::Fragment(f) => {
//                 // TODO: validate_fragment
//                 // call validate_node for each part
//             }
//             yrs::XmlOut::Text(_) => {
//                 // text is always allowed(?)
//             }
//         }
//     }

//     Ok(())
// }

mod next {

    // TODO: design this type better?
    trait DocumentFormat {
        // serdoc
        type Serialized;

        // a serialized update that can be applied
        type Update;

        fn serialize(s: Self::Serialized, doc: &mut ());
        fn deserialize(doc: ()) -> Self::Serialized;
        fn validate_update(doc: (), update: ());
        fn validate_doc(doc: ());
    }
}

// // TODO: impl
// mod prose {
//     struct SerializedProse;
// }

// mod redex {
//     struct SerializedRedex;
// }
