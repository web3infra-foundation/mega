use callisto::{mega_issue, mega_mr};

use crate::model::common::ItemKind;

pub trait ItemEntity {
    type Model;
    fn item_kind(model: Self::Model) -> ItemKind;

    fn get_id(model: &Self::Model) -> i64;
}

impl ItemEntity for mega_issue::Entity {
    type Model = mega_issue::Model;

    fn item_kind(model: Self::Model) -> ItemKind {
        ItemKind::Issue(model)
    }

    fn get_id(model: &Self::Model) -> i64 {
        model.id
    }
}

impl ItemEntity for mega_mr::Entity {
    type Model = mega_mr::Model;

    fn item_kind(model: Self::Model) -> ItemKind {
        ItemKind::Mr(model)
    }

    fn get_id(model: &Self::Model) -> i64 {
        model.id
    }
}
