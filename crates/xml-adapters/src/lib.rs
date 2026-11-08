#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

#[cfg(any(feature = "relax-ng", feature = "xpath"))]
mod position;
#[cfg(feature = "relax-ng")]
mod rng;
#[cfg(feature = "xpath")]
mod xpath;

#[cfg(any(feature = "relax-ng", feature = "xpath"))]
pub use position::Position;
#[cfg(feature = "relax-ng")]
pub use rng::RngElement;
#[cfg(feature = "xpath")]
pub use xpath::{XmlDoc, XmlNode};
