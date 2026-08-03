use crate::{
    v1::types::misc::Color,
    v2::types::components::{
        ComponentCustomId, Components, action::ButtonAction, interactive::Label,
    },
};

#[test]
fn test_macro() {
    let action = ButtonAction::Interaction {
        custom_id: ComponentCustomId("example".into()),
    };

    let color = Color::from_str_strict("#123456").unwrap();

    let components = lamprey_macros::components! {
        container() {
            text("Pick one:")
            button(label: Label::from("label"), style: Primary, action)
        }

        container(color) {
            text("Pick one:")
            button(label: "example", style: Primary, action)
        }

        details(color: None, open: false) {
            summary:
            text("hello")

            details:
            text("world")
        }

         details(open: true) {
             summary: heading(label: "Click me")
             details: text("Hidden body")
         }
    };

    // TODO: verify components structure
}

// TODO: test_macro_color_str
// container(color: "#123456") {
//     text("Pick one:")
//     button(label: "example", style: Primary, action)
// }
