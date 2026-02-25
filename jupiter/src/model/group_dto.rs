use callisto::sea_orm_active_enums::PermissionEnum;

#[derive(Debug, Clone)]
pub struct CreateGroupPayload {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourcePermissionBinding {
    pub group_id: i64,
    pub permission: PermissionEnum,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeleteGroupStats {
    pub deleted_members_count: u64,
    pub deleted_permissions_count: u64,
    pub deleted_groups_count: u64,
}
