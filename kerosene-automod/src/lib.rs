mod compiled;
mod sanitize;
mod scannable;
mod util;

#[cfg(test)]
mod test;

pub use compiled::{Compiled, Context, Router, Rule};
pub use scannable::{Scannable, ScannableTarget, Scanner};
pub use util::{Actions, Matches, RuleIdx, RuleIndexes, Scan};

pub use lamprey::v1::types::AutomodRuleId as RuleId;
pub use lamprey::v1::types::automod::{
    AutomodMediaLocation as MediaLocation, AutomodTarget as Target,
    AutomodTextLocation as TextLocation,
};
