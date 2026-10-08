pub use error::*;
pub use func::*;
pub use ptr::*;

// use runner::*;

#[macro_use]
mod inner;
mod error;
// mod runner;
mod func;
mod ptr;

#[cfg(feature = "unicorn")]
mod unicorn;

#[cfg(feature = "arm")]
pub mod arm;

#[cfg(feature = "aarch64")]
pub mod aarch64;

#[cfg(feature = "x86")]
pub mod x86;

#[cfg(feature = "x86_64")]
pub mod x86_64;

bitflags::bitflags! {
    #[derive(Debug, Default)]
    #[repr(C)]
    pub struct Perms: i32 {
        const R = 0b0001;
        const W = 0b0010;
        const X = 0b0100;

        const RW = Self::R.bits() | Self::W.bits();
        const RX = Self::R.bits() | Self::X.bits();
        const WX = Self::W.bits() | Self::X.bits();
        const RWX = Self::RW.bits() | Self::X.bits();

        const None = 0b0000;
        const READ = Self::R.bits();
        const WRITE = Self::W.bits();
        const EXEC = Self::X.bits();
        const ALL = Self::RWX.bits();
    }
}

pub struct Anyrch {}
