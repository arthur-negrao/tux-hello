use std::collections::HashSet;

use super::candidate::{CameraCandidate, DeviceId, discover_devices};

pub struct Discover {
    blacklist: HashSet<DeviceId>,
}

impl Discover {
    pub fn new() -> Self {
        Self {
            blacklist: HashSet::new(),
        }
    }

    /// Get the winner candidate
    pub fn discover(&mut self) -> Option<CameraCandidate> {
        let candidates = discover_devices(self);
        self.select_candidate(candidates)
    }

    /// Select the winner candidate using [Rules].
    fn select_candidate(
        &mut self,
        candidates: Vec<CameraCandidate>,
    ) -> Option<CameraCandidate> {
        let has_ir_camera = candidates.iter().any(|c| c.is_ir_camera);

        for candidate in candidates {
            if candidate.is_ir_camera || !has_ir_camera {
                // avoid
                if self.is_in_blacklist(&candidate.id) {
                    continue;
                } else {
                    return Some(candidate);
                }
            }
        }

        None
    }

    #[inline]
    pub(super) fn is_in_blacklist(&self, id: &DeviceId) -> bool {
        self.blacklist.contains(id)
    }

    pub(crate) fn add_to_blacklist(
        &mut self,
        candidate: &Option<CameraCandidate>,
    ) {
        if let Some(camera) = candidate {
            self.blacklist.insert(camera.id.clone());
        }
    }
}
