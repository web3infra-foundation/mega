import { useQuery } from '@tanstack/react-query'

import type { RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetClFilesList(link: string, params?: RequestParams) {
  return useQuery({
    queryKey: [...legacyApiClient.v1.getApiClFilesList().requestKey(link), params],
    queryFn: () => legacyApiClient.v1.getApiClFilesList().request(link, params),
    enabled: !!link
  })
}
