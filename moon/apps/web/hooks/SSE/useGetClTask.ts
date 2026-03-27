import { useQuery } from '@tanstack/react-query'

import { BuildEventDTO, BuildTargetDTO, RequestParams, TargetState } from '@gitmono/types/generated'

import { BuildDTO, BuildStatus, TargetDTO, TaskInfoDTO } from '@/components/ClView/components/Checks/cpns/store'
import { orionApiClient } from '@/utils/queryClient'

const toBuildStatus = (build: BuildEventDTO): BuildStatus => {
  if (!build.end_at) return 'Building'
  if (build.exit_code === 0) return 'Completed'
  return 'Failed'
}

const toTargetState = (state: string): TargetState => {
  switch (state) {
    case TargetState.Pending:
    case TargetState.Building:
    case TargetState.Completed:
    case TargetState.Failed:
    case TargetState.Interrupted:
    case TargetState.Uninitialized:
      return state
    default:
      return TargetState.Uninitialized
  }
}

export function useGetClTask(cl: string, params?: RequestParams) {
  return useQuery<TaskInfoDTO[], Error>({
    queryKey: [...orionApiClient.task.getTaskByClV2().requestKey(cl), params],
    queryFn: async () => {
      const resp = await orionApiClient.task.getTaskByClV2().request(cl, params)

      if (!resp) return []

      const tasks = Array.isArray(resp) ? resp : [resp]

      const hydratedTasks = await Promise.all(
        tasks.map(async (task): Promise<TaskInfoDTO> => {
          const [buildEvents, targets] = await Promise.all([
            orionApiClient.buildEvents
              .getBuildEventsByTaskIdV2()
              .request(task.id)
              .catch(() => [] as BuildEventDTO[]),
            orionApiClient.targets
              .getTargetsByTaskIdV2()
              .request(task.id)
              .catch(() => [] as BuildTargetDTO[])
          ])

          const buildList: BuildDTO[] = buildEvents.map((build) => ({
            args: [],
            created_at: build.start_at,
            end_at: build.end_at ?? undefined,
            exit_code: build.exit_code ?? undefined,
            id: build.id,
            output_file: build.log_output_file,
            repo: task.repo_name,
            retry_count: build.retry_count,
            start_at: build.start_at,
            status: toBuildStatus(build),
            target: '',
            task_id: build.task_id
          }))

          const mappedTargets: TargetDTO[] = targets.map((target) => ({
            builds: buildList,
            id: target.id,
            start_at: task.created_at,
            state: toTargetState(target.latest_state),
            target_path: target.path
          }))

          return {
            build_list: buildList,
            cl_id: 0,
            created_at: task.created_at,
            targets: mappedTargets,
            task_id: task.id,
            task_name: task.repo_name,
            template: ''
          }
        })
      )

      return hydratedTasks
    }
  })
}
