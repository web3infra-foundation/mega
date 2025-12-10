#[derive(Debug, Clone)]
pub struct SidebarSyncDto {
    pub id: Option<i32>,
    pub public_id: String,
    pub label: String,
    pub href: String,
    pub visible: bool,
    pub order_index: i32,
}
