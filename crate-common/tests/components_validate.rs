use std::collections::HashSet;

use lamprey_common::v1::types::{MediaId, metadata::Metadata};
use lamprey_common::v2::types::{
    components::{
        Component, ComponentCustomId, ComponentId, ComponentMedia, ComponentType, Components,
        Create,
        interactive::{ButtonAction, ButtonStyle, Choice, Dataset, InputFormat, Label, Validation},
        types::{
            Button, Checkboxes, Container, Details, Form, Gallery, Input, Justify, Media,
            Reference, Row, Select, Text,
        },
        validate::{Limits, ParseError},
    },
    media::MediaReference,
};

fn id(n: u16) -> ComponentId {
    ComponentId(n)
}

fn custom(value: &str) -> ComponentCustomId {
    ComponentCustomId(value.to_owned())
}

fn label(text: &str) -> Label {
    Label {
        text: text.to_owned(),
        description: None,
    }
}

fn component(id: u16, ty: ComponentType) -> Component {
    Component {
        id: self::id(id),
        ty,
        allow: None,
    }
}

fn text(id: u16, content: &str) -> Component {
    component(
        id,
        ComponentType::Text(Text {
            content: content.to_owned(),
        }),
    )
}

fn button(id: u16, name: &str) -> Component {
    component(
        id,
        ComponentType::Button(Button {
            label: label(name),
            style: ButtonStyle::Primary,
            action: ButtonAction::Interaction {
                custom_id: custom(name),
            },
        }),
    )
}

fn container(id: u16, children: &[u16]) -> Component {
    component(
        id,
        ComponentType::Container(Container {
            components: children.iter().map(|c| self::id(*c)).collect(),
            color: None,
            spoiler: false,
        }),
    )
}

fn row(id: u16, children: &[u16]) -> Component {
    component(
        id,
        ComponentType::Row(Row {
            components: children.iter().map(|c| self::id(*c)).collect(),
            justify: Justify::Start,
        }),
    )
}

fn details(id: u16, summary: &[u16], children: &[u16]) -> Component {
    component(
        id,
        ComponentType::Details(Details {
            open: false,
            color: None,
            summary: summary.iter().map(|c| self::id(*c)).collect(),
            children: children.iter().map(|c| self::id(*c)).collect(),
        }),
    )
}

fn form(id: u16, children: &[u16]) -> Component {
    component(
        id,
        ComponentType::Form(Form {
            custom_id: custom(&format!("form-{id}")),
            components: children.iter().map(|c| self::id(*c)).collect(),
        }),
    )
}

fn input(id: u16, validation: Validation) -> Component {
    component(
        id,
        ComponentType::Input(Input {
            custom_id: custom(&format!("input-{id}")),
            label: label("input"),
            value: None,
            placeholder: None,
            validation,
        }),
    )
}

fn select(id: u16, dataset: Dataset, validation: Validation) -> Component {
    component(
        id,
        ComponentType::Select(Select {
            custom_id: custom(&format!("select-{id}")),
            label: label("select"),
            placeholder: None,
            dataset,
            validation,
        }),
    )
}

fn checkboxes(id: u16, choices: Vec<Choice>, validation: Validation) -> Component {
    component(
        id,
        ComponentType::Checkboxes(Checkboxes {
            custom_id: custom(&format!("checkboxes-{id}")),
            label: label("checkboxes"),
            choices,
            validation,
        }),
    )
}

fn choice(value: &str) -> Choice {
    Choice {
        label: label(value),
        value: custom(value),
        default: false,
    }
}

fn tree(roots: &[u16], items: Vec<Component>) -> Create {
    Create::new(Components {
        roots: roots.iter().map(|r| id(*r)).collect(),
        items,
        media: vec![],
        metadata: Metadata::default(),
    })
}

fn errors(roots: &[u16], items: Vec<Component>) -> Vec<ParseError> {
    match tree(roots, items).parse(Limits::default_interactive()) {
        Ok(_) => vec![],
        Err(output) => output
            .errors
            .into_iter()
            .flat_map(|field| field.errors)
            .collect(),
    }
}

fn assert_error(roots: &[u16], items: Vec<Component>, expected: impl Fn(&ParseError) -> bool) {
    let errors = errors(roots, items);
    assert!(
        errors.iter().any(expected),
        "expected error not found, got: {errors:?}"
    );
}

#[test]
fn valid_tree() {
    let items = vec![container(0, &[1, 2]), text(1, "hello"), button(2, "click")];
    assert!(errors(&[0], items).is_empty());

    let items = vec![
        container(0, &[1]),
        details(1, &[2], &[3]),
        text(2, "summary"),
        text(3, "body"),
    ];
    assert!(errors(&[0], items).is_empty());
}

#[test]
fn inert_tree_rejects_interactive_components() {
    let items = vec![container(0, &[1]), button(1, "click")];
    let output = tree(&[0], items)
        .parse(Limits::default_inert())
        .unwrap_err();
    let errors: Vec<_> = output.errors.into_iter().flat_map(|f| f.errors).collect();
    assert!(matches!(
        errors.as_slice(),
        [ParseError::InteractiveNotAllowed]
    ));
}

#[test]
fn missing_root() {
    assert_error(&[], vec![], |e| matches!(e, ParseError::MissingRoot));
}

#[test]
fn duplicate_ids() {
    let items = vec![container(0, &[1]), text(1, "a"), text(1, "b")];
    assert_error(&[0], items, |e| matches!(e, ParseError::DuplicateId(_)));
}

#[test]
fn unknown_child() {
    let items = vec![container(0, &[1])];
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::UnknownComponent(_))
    });
}

#[test]
fn shared_child() {
    let items = vec![
        container(0, &[1, 2]),
        text(1, "shared"),
        container(2, &[1, 3]),
        text(3, "x"),
    ];
    assert_error(&[0], items, |e| matches!(e, ParseError::DuplicateParent(_)));
}

#[test]
fn cycle_is_rejected() {
    let items = vec![container(0, &[1]), container(1, &[0])];
    assert_error(&[0], items, |e| matches!(e, ParseError::Cycle(_)));
}

#[test]
fn depth_is_limited() {
    let mut items: Vec<Component> = (0..=32).map(|i| container(i, &[i + 1])).collect();
    items.push(container(33, &[]));
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::DepthExceeded { .. })
    });
}

#[test]
fn row_placement_and_length() {
    let items = vec![row(0, &[1]), container(1, &[])];
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::InvalidPlacement { .. })
    });

    let mut items = vec![row(0, &[1, 2, 3, 4, 5, 6])];
    for i in 1..=6 {
        items.push(button(i, &format!("button-{i}")));
    }
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::TooManyChildren { .. })
    });
}

#[test]
fn details_requires_summary() {
    let items = vec![details(0, &[], &[])];
    assert_error(&[0], items, |e| matches!(e, ParseError::MissingSummary));

    let items = vec![details(0, &[1], &[]), button(1, "nope")];
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::InvalidPlacement { .. })
    });
}

#[test]
fn form_rules() {
    let required = Validation {
        required: true,
        ..Validation::default()
    };

    let items = vec![input(1, required.clone())];
    assert_error(&[1], items, |e| {
        matches!(e, ParseError::RequiredOutsideForm)
    });

    let items = vec![form(0, &[1]), input(1, required)];
    assert!(errors(&[0], items).is_empty());

    let items = vec![form(0, &[1]), form(1, &[])];
    assert_error(&[0], items, |e| matches!(e, ParseError::FormNested));

    let items = vec![
        form(0, &[1]),
        select(1, Dataset::Dynamic, Validation::default()),
    ];
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::DynamicDatasetInForm)
    });

    let items = vec![select(0, Dataset::Dynamic, Validation::default())];
    assert!(errors(&[0], items).is_empty());

    let submit = component(
        0,
        ComponentType::Button(Button {
            label: label("submit"),
            style: ButtonStyle::Primary,
            action: ButtonAction::Submit,
        }),
    );
    assert_error(&[0], vec![submit.clone()], |e| {
        matches!(e, ParseError::SubmitOutsideForm)
    });

    let items = vec![
        form(0, &[1]),
        component(
            1,
            ComponentType::Button(Button {
                label: label("submit"),
                style: ButtonStyle::Primary,
                action: ButtonAction::Submit,
            }),
        ),
    ];
    assert!(errors(&[0], items).is_empty());
}

#[test]
fn label_length() {
    let items = vec![text(0, "a")];
    assert!(errors(&[0], items).is_empty());

    let items = vec![button(0, &"a".repeat(65))];
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::LabelTextLength { .. })
    });
}

#[test]
fn duplicate_custom_ids() {
    let mut first = button(0, "a");
    let mut second = button(1, "b");
    if let ComponentType::Button(button) = &mut first.ty {
        button.action = ButtonAction::Interaction {
            custom_id: custom("same"),
        };
    }
    if let ComponentType::Button(button) = &mut second.ty {
        button.action = ButtonAction::Interaction {
            custom_id: custom("same"),
        };
    }
    assert_error(&[0, 1], vec![first, second], |e| {
        matches!(e, ParseError::DuplicateCustomId(_))
    });
}

#[test]
fn validation_matrix() {
    let items = vec![select(
        0,
        Dataset::Dynamic,
        Validation {
            min_length: Some(1),
            ..Validation::default()
        },
    )];
    assert_error(&[0], items, |e| {
        matches!(
            e,
            ParseError::FieldNotAllowed {
                field: "min_length",
                ..
            }
        )
    });

    let items = vec![input(
        0,
        Validation {
            min_length: Some(10),
            max_length: Some(5),
            ..Validation::default()
        },
    )];
    assert_error(&[0], items, |e| {
        matches!(
            e,
            ParseError::InvalidRange {
                field: "min_length"
            }
        )
    });

    let items = vec![input(
        0,
        Validation {
            format: Some(InputFormat::Time {
                date: false,
                time: false,
            }),
            ..Validation::default()
        },
    )];
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::InvalidFormat { .. })
    });

    #[cfg(feature = "regex_validation")]
    {
        let items = vec![input(
            0,
            Validation {
                format: Some(InputFormat::Regex {
                    regex: "(".to_owned(),
                    error_message: None,
                }),
                ..Validation::default()
            },
        )];
        assert_error(&[0], items, |e| matches!(e, ParseError::InvalidRegex));
    }

    let items = vec![checkboxes(
        0,
        vec![choice("a"), choice("b")],
        Validation {
            max_values: Some(5),
            ..Validation::default()
        },
    )];
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::TooManySelected { .. })
    });
}

#[test]
fn gallery_needs_items() {
    let items = vec![component(
        0,
        ComponentType::Gallery(Gallery { items: vec![] }),
    )];
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::GalleryItemsLength { .. })
    });

    let media = ComponentMedia::new_media(MediaId::default());
    let items = vec![component(
        0,
        ComponentType::Gallery(Gallery {
            items: (0..21).map(|_| media.clone()).collect(),
        }),
    )];
    assert_error(&[0], items, |e| {
        matches!(e, ParseError::GalleryItemsLength { .. })
    });
}

#[test]
fn media_url_scheme() {
    let media = ComponentMedia::from(MediaReference::Url {
        source_url: "javascript:alert(1)".parse().unwrap(),
    });
    let items = vec![component(0, ComponentType::Media(Media { item: media }))];
    assert_error(&[0], items, |e| matches!(e, ParseError::InvalidUrl));
}

#[test]
fn references_require_the_previous_tree() {
    let items = vec![component(
        0,
        ComponentType::Reference(Reference {
            reference_id: id(42),
        }),
    )];
    assert_error(&[0], items.clone(), |e| {
        matches!(e, ParseError::UnknownComponent(_))
    });

    let previous: HashSet<_> = [id(42)].into_iter().collect();
    tree(&[0], items)
        .parse_with_previous(Limits::default_interactive(), previous)
        .unwrap();
}

#[test]
fn metadata_limits() {
    let mut create = tree(&[0], vec![text(0, "hello")]);
    let metadata = create.get_unvalidated_mut();
    for i in 0..9 {
        metadata
            .metadata
            .insert(format!("key{i}"), "value".to_owned());
    }
    let output = create.parse(Limits::default_interactive()).unwrap_err();
    let errors: Vec<_> = output.errors.into_iter().flat_map(|f| f.errors).collect();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ParseError::MetadataKeys { .. }))
    );
}

#[test]
fn validate_without_consuming() {
    let items = vec![container(0, &[1]), button(1, "click")];
    let components = tree(&[0], items);
    let components = components.unwrap_unvalidated();
    components.validate(&Limits::default_interactive()).unwrap();

    let invalid = tree(&[0], vec![container(0, &[1])]).unwrap_unvalidated();
    assert!(invalid.validate(&Limits::default_interactive()).is_err());
}
