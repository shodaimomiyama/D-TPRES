#![allow(
    clippy::uninlined_format_args,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::redundant_clone,
    clippy::unreadable_literal,
    clippy::unnested_or_patterns,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::items_after_statements,
    clippy::default_trait_access,
    clippy::trivially_copy_pass_by_ref,
    clippy::manual_string_new,
    clippy::single_match_else,
    clippy::redundant_else,
    clippy::if_not_else,
    clippy::option_if_let_else,
    clippy::manual_let_else,
    clippy::collection_is_never_read,
    clippy::redundant_closure_for_method_calls,
    clippy::range_plus_one,
    clippy::arithmetic_side_effects
)]

mod unit {
    mod actions;
    mod adapter;
    mod controller;
    mod domain;
    mod helpers;
    mod repositories;
    mod usecase;
}
