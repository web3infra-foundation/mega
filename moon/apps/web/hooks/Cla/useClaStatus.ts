import { useQuery } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function useClaStatus() {
  const api = legacyApiClient.v1.getApiUserClaStatus()

  return useQuery({
    queryKey: api.requestKey(),
    queryFn: () => api.request()
  })
}
