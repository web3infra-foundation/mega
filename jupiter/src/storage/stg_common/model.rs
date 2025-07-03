use callisto::{label, mega_issue, mega_mr};

pub enum ItemKind {
    Issue(mega_issue::Model),
    Mr(mega_mr::Model),
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
    pub link: String,
    pub username: String,
    pub item_type: String,
}
