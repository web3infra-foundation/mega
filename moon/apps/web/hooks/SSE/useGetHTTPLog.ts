import { useQuery } from '@tanstack/react-query'

import { GetTaskHistoryOutputByIdData, GetTaskHistoryOutputByIdParams, RequestParams } from '@gitmono/types/generated'

import { orionApiClient } from '@/utils/queryClient'

export function useGetHTTPLog(payload: GetTaskHistoryOutputByIdParams, params?: RequestParams) {
  return useQuery<GetTaskHistoryOutputByIdData, Error>({
    queryKey: [...orionApiClient.id.getTaskHistoryOutputById().requestKey(payload), params],
    queryFn: () => orionApiClient.id.getTaskHistoryOutputById().request(payload, params)
  })
}

export const fetchHTTPLog = (payload: GetTaskHistoryOutputByIdParams, params?: RequestParams) => {
  return orionApiClient.id.getTaskHistoryOutputById().request(payload, params)
}
