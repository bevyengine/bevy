use openxr::HandJointLocations;

pub struct HandTrackers {
    pub tracker_l: openxr::HandTracker,
    pub tracker_r: openxr::HandTracker,
}

impl HandTrackers {
    pub fn new(session: &openxr::Session<openxr::Vulkan>) -> Result<Self, crate::Error> {
        let ht = HandTrackers {
            tracker_l: session.create_hand_tracker(openxr::HandEXT::LEFT)?,
            tracker_r: session.create_hand_tracker(openxr::HandEXT::RIGHT)?,
        };

        Ok(ht)
    }
}

#[derive(Default)]
pub struct HandPoseState {
    pub left: Option<HandJointLocations>,
    pub right: Option<HandJointLocations>,
}

impl std::fmt::Debug for HandPoseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(left: {}, right: {})",
            self.left.is_some(),
            self.right.is_some()
        )
    }
}
