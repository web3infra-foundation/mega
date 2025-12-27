import { useQuery } from '@tanstack/react-query'

import { GetTaskHistoryOutputData, GetTaskHistoryOutputParams, RequestParams } from '@gitmono/types/generated'

import { orionApiClient } from '@/utils/queryClient'

export function useGetHTTPLog(query: GetTaskHistoryOutputParams, params?: RequestParams) {
  const request = orionApiClient.getTaskHistoryOutput()

  return useQuery<GetTaskHistoryOutputData, Error>({
    queryKey: [...request.requestKey(query), params],
    queryFn: () => request.request(query, params),
    enabled: Boolean(query?.task_id && query?.build_id && query?.repo)
  })
}

export const fetchHTTPLog = (query: GetTaskHistoryOutputParams, params?: RequestParams) => {
  return orionApiClient.getTaskHistoryOutput().request(query, params)
}
