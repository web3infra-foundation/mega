import { useMutation, useQueryClient } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostClaChangeSignStatus() {
  const queryClient = useQueryClient()
  const api = legacyApiClient.v1.postApiUserClaChangeSignStatus()
  const statusApi = legacyApiClient.v1.getApiUserClaStatus()

  return useMutation({
    mutationKey: api.requestKey(),
    mutationFn: () => api.request(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: statusApi.baseKey })
    }
  })
}
