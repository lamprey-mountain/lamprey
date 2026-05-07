use super::*;

#[test]
fn test_srgb_parsing() {
    // Hex short
    let c = Color::from_str("#f00").unwrap();
    assert_eq!(
        c,
        Color::Srgb(ColorSrgb {
            r: 255,
            g: 0,
            b: 0,
            alpha: None
        })
    );
    assert_eq!(c.to_string(), "#ff0000");

    // hex long with alpha
    let c = Color::from_str("#ff000080").unwrap();
    if let Color::Srgb(s) = c {
        assert_eq!(s.r, 255);
        assert_eq!(s.alpha, Some(128));
    } else {
        panic!("Wrong type");
    }

    // rgb functional
    let c = Color::from_str("rgb(255, 255, 255)").unwrap();
    assert_eq!(
        c,
        Color::Srgb(ColorSrgb {
            r: 255,
            g: 255,
            b: 255,
            alpha: None
        })
    );

    // rgba functional with float alpha
    let c = Color::from_str("rgba(0, 0, 0, 0.5)").unwrap();
    if let Color::Srgb(s) = c {
        assert_eq!(s.alpha, Some(127)); // 0.5 * 255 = 127.5, cast to 127
    } else {
        panic!("Wrong type");
    }
}

#[test]
fn test_oklch_parsing() {
    // standard format
    let c = Color::from_str("oklch(70% 0.1 120)").unwrap();
    assert_eq!(
        c,
        Color::Oklch(ColorOklch {
            l: 70.,
            c: 0.,
            h: 120.,
            alpha: None
        })
    );

    // with alpha
    let c = Color::from_str("oklch(70% 0.1 120 / 0.5)").unwrap();
    if let Color::Oklch(o) = c {
        assert_eq!(o.alpha, Some(127.));
    } else {
        panic!("Wrong type");
    }

    // formatting check
    assert!(c.to_string().contains("oklch"));
    assert!(c.to_string().contains("0.50"));
}

#[test]
fn test_named_parsing() {
    // simple name
    let c = Color::from_str("red").unwrap();
    assert_eq!(
        c,
        Color::Named(ColorNamed {
            name: ColorName::Red,
            variant: ColorVariant::default(),
            alpha: None
        })
    );

    // Name with variant
    let c = Color::from_str("blue-700").unwrap();
    assert_eq!(
        c,
        Color::Named(ColorNamed {
            name: ColorName::Blue,
            variant: ColorVariant::new(700).unwrap(),
            alpha: None
        })
    );

    // name with variant and alpha
    let c = Color::from_str("success-100:0.1").unwrap();
    if let Color::Named(n) = c {
        assert_eq!(n.name, ColorName::Success);
        assert_eq!(n.variant.value(), 100);
        assert_eq!(n.alpha, Some(25)); // 0.1 * 255 = 25.5
    } else {
        panic!("Wrong type");
    }
}

#[test]
fn test_variant_validation() {
    // Valid variants
    assert!(ColorVariant::new(100).is_ok());
    assert!(ColorVariant::new(500).is_ok());
    assert!(ColorVariant::new(900).is_ok());

    // invalid variants
    let err = ColorVariant::new(550).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidData);
    assert!(err.fields[0].message.contains("Invalid color variant"));
}

#[test]
fn test_mystery_fallback() {
    let input = "not-a-real-color-12345";
    let c = Color::from_str(input).unwrap();
    assert_eq!(c, Color::Mystery(input.to_string()));
    assert_eq!(c.to_string(), input);

    // also check malformed hex
    let c = Color::from_str("#invalid").unwrap();
    assert_eq!(c, Color::Mystery("#invalid".to_string()));
}

#[test]
fn test_roundtrip() {
    let cases = vec![
        "#ff0000",
        "#00ff00aa",
        "oklch(50% 0 200)",
        "red",
        "blue-200",
        "success:0.50",
        "danger-900:0.10",
    ];

    for case in cases {
        let parsed = Color::from_str(case).unwrap();
        let formatted = parsed.to_string();
        // NOTE: because of internal u8 rounding and hex normalization, check if
        // it parses back to the same structural value.
        let reparsed = Color::from_str(&formatted).unwrap();
        assert_eq!(parsed, reparsed, "Failed on case: {}", case);
    }
}

#[test]
fn test_empty_input() {
    let res = Color::from_str("");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, ErrorCode::InvalidData);

    let res = Color::from_str("   ");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, ErrorCode::InvalidData);
}
