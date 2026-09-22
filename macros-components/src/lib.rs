use proc_macro::TokenStream;

mod color;
mod components;
mod util;

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

/// compile time checked color
#[proc_macro]
pub fn color(input: TokenStream) -> TokenStream {
    color::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
