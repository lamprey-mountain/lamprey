use common::v1::types::document::serialized::{Serdoc, SerdocBlock, SerdocRoot};
use yrs::types::xml::{XmlElementPrelim, XmlIn};
use yrs::{Doc, GetString, Transact, XmlFragment, XmlOut, XmlTextPrelim};

pub fn doc_to_serdoc(doc: &Doc) -> Serdoc {
    let txn = doc.transact();
    let root = doc.get_or_insert_xml_fragment("doc");
    let mut blocks = Vec::new();

    for child in root.children(&txn) {
        if let XmlOut::Element(elem) = child {
            // if **elem.tag() == *"markdown" {
            if **elem.tag() == *"paragraph" {
                // TODO: verify that this works correctly
                let content = elem.get_string(&txn);
                blocks.push(SerdocBlock::Markdown { content });
            }
        }
    }

    Serdoc {
        root: SerdocRoot { blocks },
    }
}

pub fn serdoc_apply_to_doc(doc: &Doc, serdoc: &Serdoc) {
    let mut txn = doc.transact_mut();
    let root = doc.get_or_insert_xml_fragment("doc");

    // clear existing data
    let len = root.len(&txn);
    if len > 0 {
        root.remove_range(&mut txn, 0, len);
    }

    for block in &serdoc.root.blocks {
        match block {
            SerdocBlock::Markdown { content } => {
                let root_len = root.len(&txn);
                let content: XmlIn = XmlTextPrelim::new(content).into();
                root.insert(
                    &mut txn,
                    root_len,
                    XmlElementPrelim::new("paragraph", [content]),
                );
            }
        }
    }
}
