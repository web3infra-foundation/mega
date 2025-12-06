use callisto::dynamic_sidebar;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SidebarRes {
    pub id: i32,
    pub name: String,
    pub label: String,
    pub href: String,
    pub visible: bool,
    pub order_index: i32,
}

impl From<dynamic_sidebar::Model> for SidebarRes {
    fn from(value: dynamic_sidebar::Model) -> Self {
        Self {
            id: value.id,
            name: value.public_id,
            label: value.label,
            href: value.href,
            visible: value.visible,
            order_index: value.order_index,
        }
    }
}

pub type SidebarMenuListRes = Vec<SidebarRes>;

#[derive(Clone, Debug, ToSchema, Serialize, Deserialize)]
pub struct CreateSidebarPayload {
    pub public_id: String,
    pub label: String,
    pub href: String,
    pub visible: bool,
    pub order_index: i32,
}

#[derive(Clone, Debug, ToSchema, Serialize, Deserialize)]
pub struct UpdateSidebarPayload {
    pub public_id: Option<String>,
    pub label: Option<String>,
    pub href: Option<String>,
    pub visible: Option<bool>,
    pub order_index: Option<i32>,
}
