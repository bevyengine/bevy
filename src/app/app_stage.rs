use legion::schedule::Stage;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppStage {
    Update,
    Render,
}

impl Stage for AppStage {}

impl std::fmt::Display for AppStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppStage::Update => write!(f, "update"),
            AppStage::Render => write!(f, "draw"),
        }
    }
}