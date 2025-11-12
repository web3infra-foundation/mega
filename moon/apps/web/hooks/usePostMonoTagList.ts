import { useQuery } from '@tanstack/react-query'

import { PageParamsString, PostApiTagsListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

const fetchTagsList = legacyApiClient.v1.postApiTagsList()

export function usePostMonoTagList(
  data: PageParamsString = {
    additional: '/',
    pagination: { page: 1, per_page: 50 }
  },
  params?: RequestParams
) {
  return useQuery<PostApiTagsListData>({
    queryKey: [fetchTagsList.baseKey, data, params],
    queryFn: () => fetchTagsList.request(data, params)
  })
}
