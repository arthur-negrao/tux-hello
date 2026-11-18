/*
* This module keeps the business rules to choice the current device to use in
* facil recognition. This step will choice the device and store it in a cache
* file to avoid discover again.
*
* The business rules are written in: /etc/tux_hello/config.toml
*/

pub mod candidate;
pub mod discover;

pub use candidate::CameraCandidate;
pub use discover::Discover;
