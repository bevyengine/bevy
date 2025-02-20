use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MorphTargetNames {
    pub target_names: Vec<String>,
}
