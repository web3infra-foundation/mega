import { useQuery } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetClaContent() {
  const api = legacyApiClient.v1.getApiUserClaContent()

  return useQuery({
    queryKey: api.requestKey(),
    queryFn: () => api.request()
  })
}
