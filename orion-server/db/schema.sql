-- Orion schema (no CREATE DATABASE here)
CREATE TABLE IF NOT EXISTS public.builds (
  build_id    UUID PRIMARY KEY,
  output_file TEXT NOT NULL,
  exit_code   INTEGER,
  start_at    TIMESTAMPTZ NOT NULL,
  end_at      TIMESTAMPTZ,
  repo_name   TEXT NOT NULL,
  target      TEXT NOT NULL,
  arguments   TEXT NOT NULL,
  mr          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_builds_mr ON public.builds (mr);
CREATE INDEX IF NOT EXISTS idx_builds_start_at ON public.builds (start_at);
