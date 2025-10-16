import { GetApiTasksData } from "@/components/ClBox/types/mergeCheck.config";

export const useMergeChecks = (_prId: string) => {
  const checks: GetApiTasksData = [
    {
      repo_name: "frontend-app",
      status: "Success",
      arguments: "--env=production",
      build_id: "build-123",
      end_at: "2025-08-18T10:00:00Z",
      exit_code: 0,
      cl: "CL-456",
      output_file: "output.log",
      start_at: "2025-08-18T09:50:00Z",
      target: "main"
    },
    {
      repo_name: "backend-service",
      status: "Pending",
      arguments: "--mode=test",
      build_id: "build-124",
      end_at: "2025-08-18T10:15:00Z",
      exit_code: 0,
      cl: "CL-457",
      output_file: "test.log",
      start_at: "2025-08-18T10:00:00Z",
      target: "develop"
    },
    {
      repo_name: "shared-lib",
      status: "Failure",
      arguments: "--check=lint",
      build_id: "build-125",
      end_at: "2025-08-18T10:30:00Z",
      exit_code: 1,
      cl: "CL-458",
      output_file: "error.log",
      start_at: "2025-08-18T10:20:00Z",
      target: "feature"
    },
    {
      repo_name: "config-repo",
      status: "Warning",
      arguments: "--validate",
      build_id: "build-126",
      end_at: "2025-08-18T10:45:00Z",
      exit_code: 0,
      cl: "CL-459",
      output_file: "warning.log",
      start_at: "2025-08-18T10:35:00Z",
      target: "main"
    }
  ];
  // eslint-disable-next-line no-empty-function
  const refresh = () => {}

  return { checks, refresh };
};