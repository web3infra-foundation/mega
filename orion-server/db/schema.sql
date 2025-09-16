-- Orion schema (no CREATE DATABASE here)
-- Tasks table - represents a user's build request
CREATE TABLE IF NOT EXISTS public.tasks (
  id UUID PRIMARY KEY,
  mr_id BIGINT NOT NULL,
  task_name TEXT,
  template JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_tasks_mr ON public.tasks (mr_id);

CREATE INDEX IF NOT EXISTS idx_tasks_start_at ON public.tasks (created_at);

-- Builds table - represents individual builds belonging to a task
CREATE TABLE IF NOT EXISTS public.builds (
  id UUID PRIMARY KEY,
  task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  exit_code INT,
  start_at TIMESTAMPTZ NOT NULL,
  end_at TIMESTAMPTZ,
  repo TEXT NOT NULL,
  target TEXT NOT NULL,
  args TEXT [] DEFAULT NULL,
  output_file TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_builds_task_id ON public.builds (task_id);

CREATE INDEX IF NOT EXISTS idx_builds_start_at ON public.builds (start_at);