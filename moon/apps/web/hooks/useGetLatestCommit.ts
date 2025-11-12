import { useQuery } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

const query = legacyApiClient.v1.getApiLatestCommit()

export function useGetLatestCommit(path?: string, refs?: string) {
  return useQuery({
    queryKey: query.requestKey({ path, refs }),
    queryFn: () => query.request({ path, refs }),
    enabled: !!path
  })
}
