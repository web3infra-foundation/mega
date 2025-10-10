use callisto::{label, mega_cl, mega_issue};

pub enum ItemKind {
    Issue(mega_issue::Model),
    Cl(mega_cl::Model),
}

pub struct ItemDetails {
    pub item: ItemKind,
    pub labels: Vec<label::Model>,
    pub assignees: Vec<String>,
    pub comment_num: usize,
}

pub struct ListParams {
    pub status: String,
    pub author: Option<String>,
    pub labels: Option<Vec<i64>>,
    pub assignees: Option<Vec<String>>,
    pub sort_by: Option<String>,
    pub asc: bool,
}

pub struct LabelAssigneeParams {
    pub item_id: i64,
    pub item_type: String,
}
