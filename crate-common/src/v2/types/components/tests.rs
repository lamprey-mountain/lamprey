use crate::v2::types::components::{
    ComponentCustomId, Components, action::ButtonAction, builder::ComponentsBuilder,
    interactive::Label,
};

#[test]
fn test_macro() {
    let action = ButtonAction::Interaction {
        custom_id: ComponentCustomId("example".into()),
    };

    let components = lamprey_macros::components! {
        container() {
            text("Pick one:")
            button(label: Label::from("label"), style: Primary, action)
        }

        container(color: "#123456") {
            text("Pick one:")
            button(label: "example", style: Primary, action)
        }

        details() {
            summary:
            text("hello")

            children:
            text("world")
        }

         details(open: true) {
             summary: heading(label: "Click me")
             details: text("Hidden body")
         }
    };
}

#[test]
fn test_builder() {
    let components = Components::builder()
        .root(|c| {
            c.container(None, |c| {
                c.text("hello world")
                    .container(None, |c| c.text("sub container"))
                    .text("outside the container")
            })
        })
        .build();
    dbg!(components);
    panic!()
}
