use walkdir::WalkDir;

/// Gets all .wgsl files from the workspace
pub fn get_wgsl_files() -> Vec<String> {
    WalkDir::new(".")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "wgsl"))
        .map(|e| e.path().display().to_string())
        .collect()
}
