use crate::{check_result, entity_ext::generate_id, sea_orm_active_enums::CheckTypeEnum};

impl check_result::Model {
    pub fn new(
        path: &str,
        cl_link: &str,
        commit_id: &str,
        check_type_code: CheckTypeEnum,
        status: &str,
        message: &str,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: generate_id(),
            created_at: now,
            updated_at: now,
            path: path.to_owned(),
            cl_link: cl_link.to_owned(),
            commit_id: commit_id.to_owned(),
            check_type_code,
            status: status.to_owned(),
            message: message.to_owned(),
        }
    }
}
