use core::fmt;

/// sanitizes html on display
pub struct SanitizedHtml<'a>(pub &'a str);

pub trait Sanitizable {
    fn sanitized(&self) -> SanitizedHtml<'_>;
}

impl Sanitizable for String {
    fn sanitized(&self) -> SanitizedHtml<'_> {
        SanitizedHtml(self)
    }
}

impl fmt::Display for SanitizedHtml<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // write unescaped runs in bulk, only break out for special chars
        let mut last = 0;
        for (i, b) in self.0.bytes().enumerate() {
            let rep = match b {
                b'&' => "&amp;",
                b'<' => "&lt;",
                b'>' => "&gt;",
                b'"' => "&quot;",
                b'\'' => "&#39;",
                _ => continue,
            };
            f.write_str(&self.0[last..i])?;
            f.write_str(rep)?;
            last = i + 1;
        }
        f.write_str(&self.0[last..])
    }
}
