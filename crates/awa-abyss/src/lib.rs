#![allow(internal_features)]
#![feature(rustc_attrs)]
#![feature(stmt_expr_attributes)]
#![feature(get_many_mut)]

mod arena;
pub use arena::*;

pub mod linked;

#[cfg(feature = "default_linked")]
pub use linked::*;
