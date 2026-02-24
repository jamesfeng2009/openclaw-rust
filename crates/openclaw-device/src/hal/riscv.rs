//! RISC-V Hardware Abstraction Layer
//!
//! Provides hardware abstraction for RISC-V based embedded devices.

#[cfg(feature = "riscv")]
pub mod gpio;

#[cfg(feature = "riscv")]
pub mod i2c;

#[cfg(feature = "riscv")]
pub mod spi;

#[cfg(feature = "riscv")]
pub mod uart;

#[cfg(feature = "riscv")]
pub mod adc;

#[cfg(feature = "riscv")]
pub mod timer;
