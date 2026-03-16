import { useMutation, useQueryClient } from '@tanstack/react-query'

import { UpdateClaContentPayload } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostClaContent() {
  const queryClient = useQueryClient()
  const api = legacyApiClient.v1.postApiUserClaContent()
  const getApi = legacyApiClient.v1.getApiUserClaContent()

  return useMutation({
    mutationKey: api.requestKey(),
    mutationFn: (data: UpdateClaContentPayload) => api.request(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: getApi.baseKey })
    }
  })
}
