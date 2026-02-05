import { useQuery } from '@tanstack/react-query'

import type { RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

const getCommentsQuery = legacyApiClient.v1.getApiCodeReviewComments()

/**
 * 获取指定 CL 的完整评论树
 * GET /code_review/{link}/comments
 */
export function useGetComments(link: string, params?: RequestParams) {
  return useQuery({
    queryKey: [...getCommentsQuery.requestKey(link), params],
    queryFn: () => getCommentsQuery.request(link, params),
    enabled: !!link
  })
}
