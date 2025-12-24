use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema, Clone)]
pub struct Pagination {
    pub page: u64,
    pub per_page: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Pagination {
            page: 1,
            per_page: 20,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct PageParams<T> {
    pub pagination: Pagination,
    pub additional: T,
}

#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CommonPage<T> {
    pub total: u64,
    pub items: Vec<T>,
}
