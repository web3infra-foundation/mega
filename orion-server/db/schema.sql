-- Orion schema (no CREATE DATABASE here)
CREATE TABLE IF NOT EXISTS public.tasks (
  task_id     UUID PRIMARY KEY,
  build_ids   JSONB NOT NULL,
  output_files JSONB NOT NULL,
  exit_code   INTEGER,
  start_at    TIMESTAMPTZ NOT NULL,
  end_at      TIMESTAMPTZ,
  repo_name   TEXT NOT NULL,
  target      TEXT NOT NULL,
  arguments   TEXT NOT NULL,
  mr          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_mr ON public.tasks (mr);
CREATE INDEX IF NOT EXISTS idx_tasks_start_at ON public.tasks (start_at);