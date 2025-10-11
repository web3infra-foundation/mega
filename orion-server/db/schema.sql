-- Orion schema (no CREATE DATABASE here)

-- Table Definition
CREATE TABLE "public"."tasks" (
    "id" uuid NOT NULL,
    "cl_id" int8 NOT NULL,
    "task_name" varchar,
    "template" jsonb,
    "created_at" timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY ("id")
);

-- Table Definition
CREATE TABLE "public"."builds" (
    "id" uuid NOT NULL,
    "task_id" uuid NOT NULL,
    "exit_code" int4,
    "start_at" timestamptz NOT NULL,
    "end_at" timestamptz,
    "repo" varchar NOT NULL,
    "target" varchar NOT NULL,
    "args" jsonb,
    "output_file" varchar NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY ("id")
);



-- Indices
CREATE INDEX idx_tasks_created_at ON public.tasks USING btree (created_at);
ALTER TABLE "public"."builds" ADD FOREIGN KEY ("task_id") REFERENCES "public"."tasks"("id") ON DELETE CASCADE;


-- Indices
CREATE INDEX idx_builds_task_id ON public.builds USING btree (task_id);
CREATE INDEX idx_builds_start_at ON public.builds USING btree (start_at);