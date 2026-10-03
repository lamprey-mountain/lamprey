use lamprey::v2::types::media::Media;

use crate::{MediaLocation, Target, TextLocation};

/// something that can be scanned by automod
pub trait Scannable {
    /// Visits every piece of scannable text or media within the item.
    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S);
}

/// a top level scannable resource
pub trait ScannableTarget: Scannable {
    fn target(&self) -> Target;
}

/// A visitor trait for handling scanned item fields.
pub trait Scanner<'a> {
    /// visit a nested scannable item
    fn visit_scannable<S: Scannable>(&mut self, scannable: &'a S);

    /// visit a piece of text
    fn visit_text(&mut self, text: &'a str, location: TextLocation);

    /// visit some media
    // NOTE: requires the Scannable to actually have resolved media
    fn visit_media(&mut self, media: &Media, location: MediaLocation);
}

// pub struct ScannableMessageCreate<'a> {
//     create: &'a MessageCreate,
//     media: Vec<&'a Media>,
// }
