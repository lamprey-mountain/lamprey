use url::Url;

use crate::v2::types::{
    ChannelId,
    components::{ComponentCustomId, Components},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// what to do when a button is pressed
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ButtonAction {
    /// open a link in new tab
    Open { url: Url },

    /// navigate to a location in the app
    Navigate { target: Navigate },

    /// copy text to clipboard
    Copy { text: String },

    /// dispatch an interaction
    Interaction {
        /// developer-defined identifier for this component
        custom_id: ComponentCustomId,
    },

    /// submit the form the button is in
    ///
    /// the button must be inside of a `Form` component
    Submit,
    // // open various things
    // OpenPopover,
    // OpenModal,
    // OpenSidepane,
    // OpenFullpane,

    // // variables
    // VariableSet,

    // // messages
    // MessagePrefill,

    // SuggestCommand {
    //     command: String,
    // },
    // RunCommand {
    //     command: String,
    // },
}

/// what to do on hover
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum HoverAction {
    /// show these components
    Display { components: Components },
}

/// what to navigate to
// TODO: impl Display, FromStr
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Navigate {
    /// go to a channel id
    ///
    /// path: `/channel/{channel_id}`
    Channel(ChannelId),
    // TODO: see frontend/src/app/App.tsx for other valid urls
}

impl ButtonAction {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            Self::Interaction { .. } | Self::Submit
        )
    }
}
