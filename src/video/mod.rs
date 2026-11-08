//! Video pipeline: capture + hardware encode on the sender, a fan-out hub
//! that forwards the untouched encoded bitstream to every viewer, and
//! hardware decode on the receiver. The bitstream is never re-encoded,
//! so every viewer gets exactly what the sender's encoder produced.

pub mod decode;
pub mod encode;
mod hub;

pub use hub::Hub;
