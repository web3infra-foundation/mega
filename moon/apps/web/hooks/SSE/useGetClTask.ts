import { useQuery } from '@tanstack/react-query'

import { GetTasksByClData, RequestParams } from '@gitmono/types/generated'

import { orionApiClient } from '@/utils/queryClient'

export function useGetClTask(cl: number, params?: RequestParams) {
  return useQuery<GetTasksByClData, Error>({
    queryKey: [...orionApiClient.cl.getTasksByCl().requestKey(cl), params],
    queryFn: () => orionApiClient.cl.getTasksByCl().request(cl, params)
  })
}
