#![allow(internal_features)]
#![feature(rustc_attrs)]
#![feature(stmt_expr_attributes)]
#![feature(get_many_mut)]

mod arena;
pub use arena::*;
mod buffered;
pub use buffered::*;

pub mod linked;

cfg_if::cfg_if!(if #[cfg(feature = "default_buffered-linked")] {
    pub type Abyss<T> = Buffered<linked::Abyss<T>>;
} else if #[cfg(feature = "default_linked")] {
    pub use linked::Abyss;
});
