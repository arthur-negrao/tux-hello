use tracing::{info, warn};
use v4l::Device;
use v4l::capability::Flags;
use v4l::context::enum_devices;

use super::Discover;

/// A Device candidate to use in the application.
pub struct CameraCandidate {
    pub name: String,
    pub device: Device,

    /// Has a tip in the name like: IR, Infrared
    pub is_ir_camera: bool,
}

impl CameraCandidate {
    /// Create a new [`CameraCandidate`].
    fn new(device: Device, name: String) -> Self {
        let camera_name = name.to_lowercase();

        let is_ir_camera =
            camera_name.contains("ir") || camera_name.contains("infrared");

        Self {
            name,
            device,
            is_ir_camera,
        }
    }
}

/// Discover all devices available.
pub(super) fn discover_devices(
    discover: &mut Discover,
) -> Vec<CameraCandidate> {
    let nodes = enum_devices();

    let mut candidates = Vec::new();

    for node in nodes {
        let device_name = node
            .name()
            .unwrap_or("Unknown Device".to_string());
        let device_idx = node.index();

        let Ok(dev) = Device::new(node.index()) else {
            info!(
                "Can not access the device '{}' by the index: {}",
                device_name, device_idx
            );

            // jump the unavailable device
            continue;
        };

        let Ok(caps) = dev.query_caps() else {
            warn!(
                "Fail to get the capabilities from the device '{}' with the index: {}",
                device_name, device_idx
            );
            continue;
        };

        let can_capture = caps.capabilities.contains(Flags::VIDEO_CAPTURE)
            || caps
                .capabilities
                .contains(Flags::VIDEO_CAPTURE_MPLANE);

        if !can_capture {
            info!(
                "The device '{}' with the index '{}' can not capture frames.",
                device_name, device_idx
            );

            discover.add_to_blacklist_by_name(device_name);

            continue; // is not a device to capture frames
        }

        candidates.push(CameraCandidate::new(dev, device_name));
    }

    candidates
}
