import { useQuery } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetClFileTree(link: string) {
  return useQuery({
    queryKey: legacyApiClient.v1.getApiClMuiTree().requestKey(link),
    queryFn: () => legacyApiClient.v1.getApiClMuiTree().request(link),
    enabled: !!link
  })
}
