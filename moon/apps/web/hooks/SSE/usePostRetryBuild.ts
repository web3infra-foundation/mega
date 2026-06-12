import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import { PostRetryBuildV2Data, RetryBuildRequest } from '@gitmono/types/generated'

import { orionApiClient } from '@/utils/queryClient'

/**
 * Hook to retry a build via the Orion v2 retry API (`POST /v2/retry-build`).
 *
 * The server resolves the original task from `build_id`, so only `build_id` and
 * `changes` are meaningful; `cl_link` / `cl_id` / `targets` are accepted for
 * payload completeness. On success the CL task list is invalidated so the newly
 * created build event shows up in the Checks tab.
 *
 * @param cl - CL link, used to invalidate the matching task query on success.
 */
export function usePostRetryBuild(cl: string) {
  const queryClient = useQueryClient()
  const mutation = orionApiClient.retryBuild.postRetryBuildV2()

  return useMutation<PostRetryBuildV2Data, Error, RetryBuildRequest>({
    mutationFn: (data) => mutation.request(data),
    onSuccess: () => {
      toast.success('Build retry triggered')

      queryClient.invalidateQueries({
        queryKey: orionApiClient.task.getTaskByClV2().requestKey(cl)
      })
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to retry build')
    }
  })
}
