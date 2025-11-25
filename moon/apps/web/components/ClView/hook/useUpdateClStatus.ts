import { useMutation, useQueryClient } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export interface UseUpdateClStatusVariables {
  link: string
  status: string
}

export function useUpdateClStatus() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ link, status }: UseUpdateClStatusVariables) =>
      legacyApiClient.v1.postApiClStatus().request(link, { status }),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClDetail().requestKey(variables.link)
      })
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClReviewers().requestKey(variables.link)
      })
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClMergeBox().requestKey(variables.link)
      })
    }
  })
}
