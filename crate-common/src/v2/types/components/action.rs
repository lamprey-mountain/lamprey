use lamprey_macros::record;
use url::Url;

use crate::v2::types::{
    ChannelId,
    components::{ComponentCustomId, Components},
};

/// what to do when a button is pressed
#[record]
#[derive(PartialEq)]
#[serde(tag = "type")]
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
#[record]
#[serde(tag = "type")]
pub enum HoverAction {
    /// show these components
    Display { components: Components },
}

/// what to navigate to
// TODO: impl Display, FromStr
#[record]
#[derive(PartialEq, Eq)]
pub enum Navigate {
    /// go to a channel id
    ///
    /// path: `/channel/{channel_id}`
    Channel(ChannelId),
    // TODO: see frontend/src/app/App.tsx for other valid urls
}

impl ButtonAction {
    pub fn is_interactive(&self) -> bool {
        matches!(self, Self::Interaction { .. } | Self::Submit)
    }
}
