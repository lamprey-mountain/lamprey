use std::collections::HashMap;

use html5ever::{
    local_name,
    tendril::StrTendril,
    tokenizer::{Tag, TagKind},
};

// TODO: handle itemref attribute
// TODO: dont buffer all items in memory (MicrodataProcessor), use stream parsing to generate embeds directly

struct MicrodataAttrs<'a> {
    id: Option<&'a StrTendril>,
    itemid: Option<&'a StrTendril>,
    itemprop: Option<&'a StrTendril>,
    itemref: Option<&'a StrTendril>,
    itemscope: bool,
    itemtype: Option<&'a StrTendril>,

    /// parsed content, depending on the tag name
    content: Option<&'a StrTendril>,
}

impl<'a> MicrodataAttrs<'a> {
    fn from_tag(tag: &'a Tag) -> Self {
        let mut id = None;
        let mut itemid = None;
        let mut itemprop = None;
        let mut itemref = None;
        let mut itemscope = false;
        let mut itemtype = None;
        let mut content = None;

        for attr in &tag.attrs {
            match (attr.name.local.clone(), tag.name.clone()) {
                (local_name!("id"), _) => id = Some(&attr.value),
                (local_name!("itemid"), _) => itemid = Some(&attr.value),
                (local_name!("itemprop"), _) => itemprop = Some(&attr.value),
                (local_name!("itemref"), _) => itemref = Some(&attr.value),
                (local_name!("itemscope"), _) => itemscope = true,
                (local_name!("itemtype"), _) => itemtype = Some(&attr.value),

                (local_name!("content"), local_name!("meta")) => content = Some(&attr.value),
                (
                    local_name!("src"),
                    local_name!("audio")
                    | local_name!("embed")
                    | local_name!("iframe")
                    | local_name!("img")
                    | local_name!("source")
                    | local_name!("track")
                    | local_name!("video"),
                ) => content = Some(&attr.value),
                (
                    local_name!("href"),
                    local_name!("a") | local_name!("area") | local_name!("link"),
                ) => content = Some(&attr.value),
                (local_name!("data"), local_name!("object")) => content = Some(&attr.value),
                (local_name!("value"), local_name!("data") | local_name!("meter")) => {
                    content = Some(&attr.value)
                }
                (local_name!("datetime"), local_name!("time")) => content = Some(&attr.value),

                _ => {}
            }
        }

        MicrodataAttrs {
            id,
            itemid,
            itemprop,
            itemref,
            itemscope,
            itemtype,
            content,
        }
    }
}

#[derive(Debug)]
enum ElementState {
    Scope(usize),
    TextProp {
        parent_idx: usize,
        names: Vec<String>,
        text: String,
    },
    None,
}

#[derive(Debug, Default)]
pub struct MicrodataProcessor {
    items: Vec<MicrodataItem>,

    /// a map from html `id` to an item's index in `items`
    id_to_idx: HashMap<String, usize>,

    /// top level microdata items
    toplevel: Vec<usize>,

    /// the current html element stack
    item_stack: Vec<ElementState>,
}

#[derive(Debug)]
pub struct MicrodataItem {
    pub item_id: Option<String>,
    pub item_type: Vec<String>,
    pub item_props: HashMap<String, Vec<MicrodataProperty>>,
}

#[derive(Debug, Clone)]
pub enum MicrodataProperty {
    Item(usize),
    Str(String),
}

impl MicrodataProcessor {
    pub fn handle_tag(&mut self, tag: &Tag) {
        if tag.kind == TagKind::StartTag {
            let attrs = MicrodataAttrs::from_tag(&tag);

            let prop_names = attrs
                .itemprop
                .map(|s| {
                    s.split_whitespace()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if attrs.itemscope {
                // create a new scope
                let item = MicrodataItem {
                    item_type: attrs
                        .itemtype
                        .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
                        .unwrap_or_default(),
                    item_props: HashMap::new(),
                    item_id: attrs.itemid.map(|s| s.to_string()),
                };
                self.items.push(item);
                let new_idx = self.items.len() - 1;

                if let Some(id) = attrs.id.map(|s| s.to_string()) {
                    self.id_to_idx.insert(id, new_idx);
                }

                if !prop_names.is_empty() {
                    if let Some(parent_idx) = self.current_idx() {
                        for name in prop_names {
                            self.items[parent_idx]
                                .item_props
                                .entry(name)
                                .or_default()
                                .push(MicrodataProperty::Item(new_idx));
                        }
                    }
                } else {
                    self.toplevel.push(new_idx);
                }

                self.item_stack.push(ElementState::Scope(new_idx));
            } else if !prop_names.is_empty() {
                if let Some(parent_idx) = self.current_idx() {
                    if let Some(content) = attrs.content {
                        for name in prop_names {
                            self.items[parent_idx]
                                .item_props
                                .entry(name)
                                .or_default()
                                .push(MicrodataProperty::Str(content.to_string()));
                        }
                        self.item_stack.push(ElementState::None);
                    } else {
                        self.item_stack.push(ElementState::TextProp {
                            parent_idx,
                            names: prop_names,
                            text: String::new(),
                        });
                    }
                } else {
                    self.item_stack.push(ElementState::None);
                }
            } else {
                self.item_stack.push(ElementState::None);
            }
        }

        if tag.kind == TagKind::EndTag || (tag.kind == TagKind::StartTag && tag.self_closing) {
            if let Some(state) = self.item_stack.pop() {
                if let ElementState::TextProp {
                    parent_idx,
                    names,
                    text,
                } = state
                {
                    for name in names {
                        self.items[parent_idx]
                            .item_props
                            .entry(name)
                            .or_default()
                            .push(MicrodataProperty::Str(text.trim().to_string()));
                    }
                }
            }
        }
    }

    pub fn handle_text(&mut self, text: &StrTendril) {
        for state in self.item_stack.iter_mut() {
            if let ElementState::TextProp {
                text: prop_text, ..
            } = state
            {
                prop_text.push_str(text);
            }
        }
    }

    #[inline]
    fn current_idx(&self) -> Option<usize> {
        self.item_stack.iter().rev().find_map(|state| {
            if let ElementState::Scope(idx) = state {
                Some(*idx)
            } else {
                None
            }
        })
    }
}
