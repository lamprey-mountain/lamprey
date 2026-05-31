use common::v1::types::components::{Component, ComponentCanonical, ComponentId, ComponentType};
use common::v1::types::document::serialized::Serdoc;
use yrs::types::xml::{XmlElementPrelim, XmlIn};
use yrs::{Doc, GetString, Transact, XmlFragment, XmlOut, XmlTextPrelim};

pub fn doc_to_serdoc(doc: &Doc) -> Serdoc {
    let root = doc.get_or_insert_xml_fragment("doc");
    let txn = doc.transact();
    let mut components = Vec::new();
    let mut next_id = 0;

    for child in root.children(&txn) {
        if let XmlOut::Element(elem) = child {
            // if **elem.tag() == *"markdown" {
            if **elem.tag() == *"paragraph" {
                // TODO: verify that this works correctly
                let content = elem.get_string(&txn);
                components.push(Component {
                    id: ComponentId(next_id),
                    ty: ComponentType::Text { content },
                    allow: None,
                });
                next_id += 1;
            }
        }
    }

    Serdoc { components }
}

pub fn serdoc_apply_to_doc(doc: &Doc, components: &[ComponentCanonical]) {
    let mut txn = doc.transact_mut();
    let root = doc.get_or_insert_xml_fragment("doc");

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
