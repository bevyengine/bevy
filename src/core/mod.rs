use legion::schedule::Stage;

pub mod time;
pub use time::Time;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApplicationStage {
    Update,
    Render,
}

impl Stage for ApplicationStage {}

impl std::fmt::Display for ApplicationStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicationStage::Update => write!(f, "update"),
            ApplicationStage::Render => write!(f, "draw"),
        }
    }
}
