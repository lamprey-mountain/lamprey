use proc_macro::TokenStream;

mod components;
mod diff;
mod endpoint;
mod endpoint_new;
mod handler;
mod handlers_new;
mod ids;
mod parse;
mod record;

#[proc_macro_derive(Diff, attributes(diff))]
pub fn derive_diff(input: TokenStream) -> TokenStream {
    diff::expand_diff_derive(input)
}

#[proc_macro_attribute]
pub fn record(_args: TokenStream, input: TokenStream) -> TokenStream {
    record::expand(input)
}

/// macro to generate components
///
/// ## usage
///
/// - most components use `type(attr: value, foo: bar) { <children> }`
/// - a set of children can be prefixed with `section:` for stuff like details
/// - `text` is special and is always in the form `text(expression)` where `expression` is your text content
///
/// ## example
///
/// ```rs
/// let action = ButtonAction::Interaction {
///     custom_id: ComponentCustomId("example".into()),
/// };
///
/// let components = components! {
///     container() {
///         text("Pick one:")
///         button(label: Label::from("label"), style: Primary, action)
///     }
///
///     container(color: "#123456") {
///         text("Pick one:")
///         button(label: "example", style: Primary, action)
///     }
///
///     details() {
///         summary:
///         text("hello")
///
///         children:
///         text("world")
///     }
///
///      details(open: true) {
///          summary: heading(label: "Click me")
///          details: text("Hidden body")
///      }
/// };
/// ```
#[proc_macro]
pub fn components(input: TokenStream) -> TokenStream {
    components::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[cfg(not(feature = "use_new_endpoint_macro"))]
#[proc_macro_attribute]
pub fn endpoint(args: TokenStream, item: TokenStream) -> TokenStream {
    endpoint::expand(args.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn handler(args: TokenStream, item: TokenStream) -> TokenStream {
    handler::expand(args.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn endpoint_new(args: TokenStream, item: TokenStream) -> TokenStream {
    endpoint_new::expand(args.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[cfg(feature = "use_new_endpoint_macro")]
#[proc_macro_attribute]
pub fn endpoint(args: TokenStream, item: TokenStream) -> TokenStream {
    endpoint_new::expand(args.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn handler_new(args: TokenStream, item: TokenStream) -> TokenStream {
    handlers_new::expand(args.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

macro_rules! define_id_macro {
    ($name:ident, $ty:literal) => {
        #[proc_macro]
        pub fn $name(input: TokenStream) -> TokenStream {
            let lit = syn::parse_macro_input!(input as syn::LitStr);
            ids::expand_typed_id(lit, $ty).into()
        }
    };
}

define_id_macro!(user_id, "UserId");
define_id_macro!(room_id, "RoomId");
define_id_macro!(channel_id, "ChannelId");
define_id_macro!(message_id, "MessageId");
define_id_macro!(role_id, "RoleId");
define_id_macro!(media_id, "MediaId");
define_id_macro!(session_id, "SessionId");
define_id_macro!(audit_log_entry_id, "AuditLogEntryId");
define_id_macro!(embed_id, "EmbedId");
define_id_macro!(tag_id, "TagId");
define_id_macro!(report_id, "ReportId");
define_id_macro!(redex_id, "RedexId");
define_id_macro!(call_id, "CallId");
define_id_macro!(emoji_id, "EmojiId");
define_id_macro!(application_id, "ApplicationId");
define_id_macro!(notification_id, "NotificationId");
define_id_macro!(sfu_id, "SfuId");
define_id_macro!(automod_rule_id, "AutomodRuleId");
define_id_macro!(webhook_id, "WebhookId");
define_id_macro!(calendar_event_id, "CalendarEventId");
define_id_macro!(harvest_id, "HarvestId");
define_id_macro!(document_branch_id, "DocumentBranchId");
define_id_macro!(document_tag_id, "DocumentTagId");
define_id_macro!(connection_id, "ConnectionId");
