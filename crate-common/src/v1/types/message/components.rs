#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::misc::Color;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageComponent {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: MessageComponentType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessageComponentType {
    /// a clickable button
    Button {
        label: String,
        style: ButtonStyle,

        /// required for non link buttons
        custom_id: Option<String>,

        /// what to link to, iff style == `Link`
        url: Option<Url>,
    },

    /// a group of other components
    Container {
        components: Vec<MessageComponent>,
        color: Option<Color>,
    },

    /// markdown text
    Text { content: String },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ButtonStyle {
    #[default]
    Primary,
    Secondary,
    Danger,

    /// a link to another page
    Link,
}

impl MessageComponentType {
    pub fn is_interactive(&self) -> bool {
        match self {
            MessageComponentType::Button { style, .. } => style != &ButtonStyle::Link,
            MessageComponentType::Container { components, .. } => {
                components.iter().any(|c| c.ty.is_interactive())
            }
            MessageComponentType::Text { .. } => false,
        }
    }

    pub fn validate(&self) -> Vec<String> {
        match self {
            MessageComponentType::Button {
                style,
                url,
                label,
                custom_id,
                ..
            } => {
                let err_custom_id_len = if let Some(custom_id) = &custom_id {
                    if custom_id.is_empty() {
                        vec!["custom_id cannot be empty".to_owned()]
                    } else if custom_id.len() > 256 {
                        vec!["custom_id can have up to 256 chars".to_owned()]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };

                let err_labels = if label.is_empty() {
                    vec!["label cannot be empty".to_owned()]
                } else if label.len() > 256 {
                    vec!["label can have up to 256 chars".to_owned()]
                } else {
                    vec![]
                };

                let err_links = match (url.is_some(), style == &ButtonStyle::Link) {
                    (true, false) => vec!["only link buttons can have a url".to_owned()],
                    (false, true) => vec!["link buttons must have a url".to_owned()],
                    (_, _) => vec![],
                };

                let err_custom_id = if custom_id.is_some() && style == &ButtonStyle::Link {
                    vec!["link button cannot have custom_id".to_owned()]
                } else {
                    vec![]
                };

                let mut errs = vec![];
                errs.extend(err_custom_id_len);
                errs.extend(err_labels);

                errs.extend(err_links);
                errs.extend(err_custom_id);
                errs
            }
            MessageComponentType::Container { components, .. } => {
                if components.is_empty() {
                    vec!["containers cannot be empty".to_owned()]
                } else if components.len() > 10 {
                    vec!["containers can have up to 10 components".to_owned()]
                } else {
                    let mut errs = vec![];
                    for c in components {
                        if matches!(c.ty, MessageComponentType::Container { .. }) {
                            errs.push("containers can only contain type: Button".to_owned())
                        }

                        errs.extend(c.ty.validate());
                    }
                    errs
                }
            }
            MessageComponentType::Text { content: text } => {
                if text.is_empty() {
                    vec!["text cannot be empty".to_owned()]
                } else if text.len() > 8192 {
                    vec!["text can have up to 8192 chars".to_owned()]
                } else {
                    vec![]
                }
            }
        }
    }
}
