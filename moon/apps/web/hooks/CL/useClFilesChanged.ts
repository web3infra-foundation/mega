import { useQuery } from '@tanstack/react-query'

import type { PageParamsString, PostApiClFilesChangedData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useClFilesChanged(link: string, data: PageParamsString, params?: RequestParams) {
  return useQuery<PostApiClFilesChangedData>({
    queryKey: [...legacyApiClient.v1.postApiClFilesChanged().requestKey(link), data, params],
    queryFn: () => legacyApiClient.v1.postApiClFilesChanged().request(link, data, params)
  })
}
