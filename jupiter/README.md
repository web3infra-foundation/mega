## Jupiter — Storage Engine

`jupiter` provides the database storage layer for Mega: SeaORM services, query helpers, and `jupiter/model` storage assembly types.

Generated entities live in [`callisto/`](callisto/). Schema migrations live in the separate [`jupiter-migrate`](../jupiter-migrate/) crate — see [jupiter-migrate/README.md](../jupiter-migrate/README.md) for generate/apply workflow.

`ceres` and `mono` consume `jupiter::storage::Storage`; HTTP DTO mapping belongs in `ceres/model`, not in routers.
