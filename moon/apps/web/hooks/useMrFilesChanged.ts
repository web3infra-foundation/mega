import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { PostApiMrFilesChangedData, PageParamsString, RequestParams } from '@gitmono/types'

export function useMrFilesChanged(
  link: string,
  data: PageParamsString,
  params?: RequestParams
) {
  return useQuery<PostApiMrFilesChangedData>({
    queryKey: [
      ...legacyApiClient.v1.postApiMrFilesChanged().requestKey(link),
      data,
      params
    ],
    queryFn: () =>
      legacyApiClient.v1.postApiMrFilesChanged().request(link, data, params)
  })
}