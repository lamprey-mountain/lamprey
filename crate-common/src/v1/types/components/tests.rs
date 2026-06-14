#[cfg(test)]
mod tests {
    use crate::v1::types::components::*;
    use crate::v1::types::error::{ApiResult, ErrorCode};
    use crate::v2::types::media::{Media, MediaErrorReason};

    fn parse_comp(
        create: ComponentCreate,
        prev: Option<&Components<Canonical>>,
    ) -> ApiResult<Component<Canonical>> {
        let components = Components {
            inner: vec![create],
        };
        let parsed = components.parse(prev, &|mr| {
            // NOTE: create a dummy piece of media
            Ok(Media::errored(
                mr.media_id().unwrap(),
                (*mr.media_id().unwrap()).into(),
                MediaErrorReason::NotFound,
            ))
        })?;
        Ok(parsed.inner.into_iter().next().unwrap())
    }

    #[test]
    fn test_parse_simple() {
        let create = ComponentCreate {
            id: None,
            ty: ComponentType::Text {
                content: "hello".to_owned(),
            },
            allow: None,
        };
        let parsed = parse_comp(create, None).unwrap();
        assert_eq!(parsed.id, ComponentId(0));
        if let ComponentType::Text { content } = parsed.ty {
            assert_eq!(content, "hello");
        } else {
            panic!("expected text");
        }
    }

    #[test]
    fn test_id_allocation() {
        let create = ComponentCreate {
            id: None,
            ty: ComponentType::Container {
                components: vec![
                    Component {
                        id: Some(ComponentId(10)),
                        ty: ComponentType::Button {
                            label: "btn1".to_owned(),
                            style: ButtonStyle::Primary,
                            custom_id: ComponentCustomId("c1".to_owned()),
                        },
                        allow: None,
                    },
                    Component {
                        id: None,
                        ty: ComponentType::Button {
                            label: "btn2".to_owned(),
                            style: ButtonStyle::Primary,
                            custom_id: ComponentCustomId("c2".to_owned()),
                        },
                        allow: None,
                    },
                ],
                color: None,
            },
            allow: None,
        };
        let parsed = parse_comp(create, None).unwrap();
        assert_eq!(parsed.id, ComponentId(0));
        if let ComponentType::Container { components, .. } = parsed.ty {
            assert_eq!(components[0].id, ComponentId(10));
            assert_eq!(components[1].id, ComponentId(1)); // 0 was used, next is 1
        }
    }

    #[test]
    fn test_reference_move() {
        let prev: Component<Canonical> = Component {
            id: ComponentId(0),
            ty: ComponentType::Text {
                content: "old".to_owned(),
            },
            allow: None,
        };
        let create = ComponentCreate {
            id: Some(ComponentId(0)),
            ty: ComponentType::Reference {
                reference_id: ComponentId(0),
            },
            allow: None,
        };
        let parsed = parse_comp(create, Some(&Components { inner: vec![prev] })).unwrap();
        assert_eq!(parsed.id, ComponentId(0));
        if let ComponentType::Text { content } = parsed.ty {
            assert_eq!(content, "old");
        }
    }

    #[test]
    fn test_reference_clone() {
        let prev: Component<Canonical> = Component {
            id: ComponentId(0),
            ty: ComponentType::Text {
                content: "old".to_owned(),
            },
            allow: None,
        };
        let create = ComponentCreate {
            id: None,
            ty: ComponentType::Reference {
                reference_id: ComponentId(0),
            },
            allow: None,
        };
        let parsed = parse_comp(create, Some(&Components { inner: vec![prev] })).unwrap();
        assert_ne!(parsed.id, ComponentId(0));
        assert_eq!(parsed.id, ComponentId(1));
        if let ComponentType::Text { content } = parsed.ty {
            assert_eq!(content, "old");
        }
    }

    #[test]
    fn test_validation_depth() {
        fn make_nested(depth: usize) -> ComponentCreate {
            if depth == 0 {
                ComponentCreate {
                    id: None,
                    ty: ComponentType::Text {
                        content: "deep".to_owned(),
                    },
                    allow: None,
                }
            } else {
                ComponentCreate {
                    id: None,
                    ty: ComponentType::Section {
                        components: vec![Component {
                            id: None,
                            ty: make_nested(depth - 1).ty,
                            allow: None,
                        }],
                        color: None,
                    },
                    allow: None,
                }
            }
        }

        let create = make_nested(32); // Max depth is 32
        let parsed = parse_comp(create, None).unwrap();
        assert!(parsed.ty.validate().is_err());
    }

    #[test]
    fn test_validation_nesting() {
        let create = ComponentCreate {
            id: None,
            ty: ComponentType::Container {
                components: vec![Component {
                    id: None,
                    ty: ComponentType::Text {
                        content: "invalid".to_owned(),
                    },
                    allow: None,
                }],
                color: None,
            },
            allow: None,
        };
        let parsed = parse_comp(create, None).unwrap();
        let err = parsed.ty.validate().unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidData);
        assert!(
            err.fields
                .iter()
                .any(|f| f.message.contains("only contain Buttons or LinkButtons"))
        );
    }
}
