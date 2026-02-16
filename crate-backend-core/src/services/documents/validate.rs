// TODO: validating documents

use std::ops::Deref;

use yrs::{Doc, Transact, Xml, XmlFragment};

use crate::{Error, Result};

/// validates this document
pub fn validate(doc: &Doc) -> Result<()> {
    // TODO: validate that no other fragments exist?
    let xml = doc.get_or_insert_xml_fragment("content");
    let tx = doc.transact();

    for c in xml.children(&tx) {
        // TODO: move into fn validate_node()
        match c {
            yrs::XmlOut::Element(e) => {
                // TODO: move into fn validate_element()
                // let attrs: Vec<_> = e.attributes(tx).collect();
                // for attr in e.attributes(tx) {

                // }
                // attrs[0].1.
                // match attrs[0].1 {
                //     yrs::Out::Any(any) => todo!(),
                //     yrs::Out::YText(text_ref) => todo!(),
                //     yrs::Out::YArray(array_ref) => todo!(),
                //     yrs::Out::YMap(map_ref) => todo!(),
                //     yrs::Out::YXmlElement(xml_element_ref) => todo!(),
                //     yrs::Out::YXmlFragment(xml_fragment_ref) => todo!(),
                //     yrs::Out::YXmlText(xml_text_ref) => todo!(),
                //     yrs::Out::YDoc(doc) => todo!(),
                //     yrs::Out::UndefinedRef(branch_ptr) => todo!(),
                // }

                // minimal format for now; only has root and markdown. embedded media and such will come later.
                match e.tag().deref() {
                    "root" => {
                        // no attrs
                        // can only contain markdown elements
                    }
                    "markdown" => {
                        // no attrs
                        // can only contain text
                    }
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

    Ok(())
}
