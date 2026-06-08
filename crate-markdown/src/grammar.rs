// TODO(future): crate for glr parsing
// TODO: macro to generate this?
// NOTE: maybe have two tables; a block and an inline table?

use crate::prelude::*;

// pub const MARKDOWN: Table = todo!();

/// parse table
pub struct Table {
    // /// The action table: defines what to do (Shift, Reduce, Accept, Error)
    // /// given the current state and the lookahead token.
    /// defines what to do given the current state and lookahead token
    // actions: &'static [u16],
    actions: Box<[Action]>,

    /// which state to transition to after reducing a rule
    gotos: Box<[u16]>,
    // /// A space-optimization table. If a state only has one valid reduction,
    // /// it is stored here to omit it from the main `actions` table.
    // pub default_reduces: &'static [u16],
    /// rule metadata for reductions
    rules: &'static [Rule],
}

/// A single reduction rule.
pub struct Rule {
    // /// The kind of node being reduced.
    /// The kind of node to create.
    pub kind: NodeKind,

    /// How many tokens/nodes to pop off the GLR stack.
    pub pop_count: u8,
}

pub enum Action {
    // {
    //     // /// Strict conversion: Replace the token type completely.
    //     // Specialize(u16),
    //     // /// Ambient conversion: The token can act as both the original identifier
    //     // /// AND the specialized keyword, spinning off GLR branches[cite: 145].
    //     // Extend(u16),
    //     /// push this token onto the stack
    //     Shift { new_state: u16 },

    //     /// pop n tokens and create a new one
    //     Reduce {
    //         // rule_id: u16,
    //         n: u16,
    //         new_kind: NodeKind,
    //         // target_node_id: u16,
    //     },
    //     // Stay,
    // }
}

impl Table {
    // fn what_do_i_do(&self, state: State, token: TokenKind) -> Action{todo!()}

    //     /// Looks up the current state in the table metadata to generate the mask.
    //     pub fn get_expected_tokens(&self, state_id: u16) -> TokenMask {
    //         // In a packed array format, we look up the offset dedicated to this state.
    //         // For example, if each state has a dedicated u64 value mapped out:
    //         let index = state_id as usize;

    //         // This array is compiled completely offline by your build generator tools.
    //         let raw_bits = self.state_to_token_mask_table[index];

    //         TokenMask { bits: raw_bits }
    //     }
}

// /// Bit-packed actions decoded at runtime during `ParseContext::advance()`.
// pub enum DecodedAction {
//     Shift { next_state: u16 },
//     Reduce { rule_id: u16, len: u16, target_node_id: u16 },
//     Stay { lookahead_bits: u16 },
//     Accept,
//     Fail,
// }

/// bitset representing which terminal tokens are valid for this state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenMask {
    bits: u64,
}

impl TokenMask {
    /// Checks if a specific token ID is allowed by this mask.
    #[inline]
    pub fn allows(&self, token: TokenKind) -> bool {
        let token_id = token as u8;
        if token_id >= 64 {
            todo!("handle this")
        }
        (self.bits & (1 << token_id)) != 0
    }
}

/// the current parse state
pub struct State(u16);

// pub struct Stack {
//     state_id: u32,
//     // TODO: symbol stacks, lookaheads, etc.
// }

// pub struct ParseContext {
//     /// all current glr branches
//     ///
//     /// contains one item if unambiguous
//     stacks: Vec<Stack>,
// }
