/*
* A module to setup the camera device after the discover. This step is used to
* adjust the camera's output format and activate the IR emitter (if the pc has).
*/

pub mod setup;

pub use setup::configure_camera;
