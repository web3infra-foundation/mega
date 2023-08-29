ALTER TABLE IF EXISTS public.git
    RENAME TO mr_object;

ALTER TABLE IF EXISTS public.obj_data
    RENAME TO git_obj;

CREATE TABLE IF NOT EXISTS "mr_info" (
  "id" SERIAL PRIMARY KEY,
  "mr_id" BIGINT NOT NULL,
  "mr_msg" VARCHAR(255) NOT NULL,
  "mr_date" TIMESTAMP NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);