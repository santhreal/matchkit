//! Main integration test runner for matchkit test suite.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreadable_literal,
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::ptr_as_ptr,
    clippy::borrow_as_ptr,
    clippy::ref_as_ptr,
    clippy::cast_ptr_alignment,
    clippy::useless_vec,
    clippy::items_after_statements,
    clippy::io_other_error,
    clippy::stable_sort_primitive,
    clippy::unnecessary_wraps,
    clippy::single_char_pattern,
    clippy::cast_sign_loss,
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::len_zero,
    clippy::elidable_lifetime_names,
    missing_docs
)]

#[path = "../adversarial/mod.rs"]
mod adversarial;
#[path = "../concurrent/mod.rs"]
mod concurrent;
mod depth;
#[path = "../integration/mod.rs"]
mod integration;
#[path = "../property/mod.rs"]
mod property;
#[path = "../regression/mod.rs"]
mod regression;
#[path = "mod.rs"]
mod unit;
