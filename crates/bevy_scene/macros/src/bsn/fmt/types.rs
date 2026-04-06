pub struct Edit {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub original_text: String,
    pub new_text: String,
}

pub struct BsnVisitor<'a> {
    pub source: &'a str,
    pub edits: Vec<Edit>,
}
