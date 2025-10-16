import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { PostApiClFilesChangedData, PageParamsString, RequestParams } from '@gitmono/types'

export function useClFilesChanged(
  link: string,
  data: PageParamsString,
  params?: RequestParams
) {
  return useQuery<PostApiClFilesChangedData>({
    queryKey: [
      ...legacyApiClient.v1.postApiClFilesChanged().requestKey(link),
      data,
      params
    ],
    queryFn: () =>
      legacyApiClient.v1.postApiClFilesChanged().request(link, data, params)
  })
}