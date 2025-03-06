use std::borrow::Cow;

/// trim and truncate a string to a maximum length, while attempting to avoid
/// splitting in the middle of a word
// i should probably also look at using graphemes vs chars; using chars gives me
// a hard upper bound on the amount of memory though
pub fn truncate(a: &str, max_len: usize) -> &str {
    let a = a.trim();
    match a.char_indices().nth(max_len) {
        Some((idx, ch)) => {
            if ch.is_alphanumeric() {
                let b = &a[0..idx]
                    .trim_end_matches(|c: char| c.is_alphanumeric())
                    .trim_end();
                if b.is_empty() {
                    &a[0..idx]
                } else {
                    b
                }
            } else {
                &a[0..idx]
            }
        }
        None => a,
    }
}

/// truncate some text, adding ellipsis at the end if its too long
pub fn truncate_with_ellipsis(a: &str, max_len: usize) -> Cow<'_, str> {
    let trunc = truncate(a, max_len - 3);
    if trunc.len() == a.len() {
        Cow::Borrowed(a)
    } else {
        Cow::Owned(format!("{trunc}..."))
    }
}

/// truncate a filename to a maximum length, while attempting to retain its
/// extension
pub fn truncate_filename(a: &str, max_len: usize) -> Cow<'_, str> {
    match dbg!(a.rfind('.')) {
        Some(idx) => {
            let ext = dbg!(&a[idx..]);
            if ext.len() > max_len {
                // the file extension would be mangled anyways
                Cow::Borrowed(truncate(a, max_len))
            } else if ext.len() == max_len {
                // remove extension to avoid returning dotfile
                Cow::Borrowed(truncate(a, max_len))
            } else {
                let name = dbg!(truncate(&a[0..idx], dbg!(max_len - ext.len())));
                Cow::Owned(format!("{}{}", name, ext))
            }
        }
        _ => Cow::Borrowed(truncate(a, max_len)),
    }
}

// first time experimenting with chatgpt for tests
// it's... okay. the tests needed quite a few corrections
// yeah idk if i'll do this anymore
#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn test_truncate() {
        // Shorter than max length
        assert_eq!(truncate("hello", 10), "hello");

        // Exactly max length
        assert_eq!(truncate("hello", 5), "hello");

        // Longer than max length, no spaces
        assert_eq!(truncate("hello_world", 5), "hello");

        // Longer than max length, with spaces
        assert_eq!(truncate("hello world", 7), "hello");

        // Cuts in the middle of a word
        assert_eq!(truncate("hello world", 8), "hello");

        // Cuts at a space
        assert_eq!(truncate("hello world", 6), "hello");

        // Only spaces
        assert_eq!(truncate("     ", 3), "");

        // Mixed alphanumeric and symbols
        assert_eq!(truncate("hello!world", 7), "hello!");

        // UTF-8 characters
        assert_eq!(truncate("привет мир", 7), "привет");
    }

    #[test]
    fn test_truncate_ellipsis() {
        assert_eq!(
            truncate_with_ellipsis("irestn", 10),
            Cow::Borrowed("irestn")
        );
        assert_eq!(
            truncate_with_ellipsis("irestnr", 10),
            Cow::Borrowed("irestnr")
        );
        assert_eq!(
            truncate_with_ellipsis("irestnri", 10),
            Cow::Borrowed("irestnr...")
        );
        assert_eq!(
            truncate_with_ellipsis("irestnrie", 10),
            Cow::Borrowed("irestnr...")
        );
        assert_eq!(
            truncate_with_ellipsis("irestnries", 10),
            Cow::Borrowed("irestnr...")
        );
        assert_eq!(
            truncate_with_ellipsis("irestnriestnoaeno", 10),
            Cow::Borrowed("irestnr...")
        );
    }

    #[test]
    fn test_truncate_filename() {
        // No extension, shorter than max length
        assert_eq!(truncate_filename("file", 10), Cow::Borrowed("file"));

        // No extension, longer than max length
        assert_eq!(
            truncate_filename("longfilename", 8),
            Cow::Borrowed("longfile")
        );

        // With extension, shorter than max length
        assert_eq!(truncate_filename("file.txt", 10), Cow::Borrowed("file.txt"));

        // With extension, longer than max length
        assert_eq!(
            truncate_filename("verylongfilename.txt", 13),
            Cow::Borrowed("verylongf.txt")
        );

        // Long extension, shorter than max length
        assert_eq!(
            truncate_filename("file.longextension", 20),
            Cow::Borrowed("file.longextension")
        );

        // Long extension, longer than max length
        assert_eq!(
            truncate_filename("verylongfilename.withverylongextension", 14),
            Cow::<str>::Owned("verylongfilena".to_string())
        );

        // long extension 2
        assert_eq!(
            truncate_filename("verylongfilename.withverylongextension", 33),
            Cow::<str>::Owned("verylongfil.withverylongextension".to_string())
        );

        // Edge case: max_len shorter than extension
        assert_eq!(
            truncate_filename("file.verylongextension", 5),
            Cow::Borrowed("file.")
        );

        // No extension, max_len 0
        assert_eq!(truncate_filename("file", 0), Cow::Borrowed(""));

        // UTF-8 characters in filename
        assert_eq!(
            truncate_filename("привет.txt", 8),
            Cow::Borrowed("прив.txt")
        );
    }
}
