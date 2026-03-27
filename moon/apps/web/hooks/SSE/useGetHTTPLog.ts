import { useQuery } from '@tanstack/react-query'

import { GetBuildsLogsV2Data, RequestParams } from '@gitmono/types/generated'

import { orionApiClient } from '@/utils/queryClient'

export function useGetHTTPLog(buildId: string, params?: RequestParams) {
  const request = orionApiClient.builds.getBuildsLogsV2()

  return useQuery<GetBuildsLogsV2Data, Error>({
    queryKey: [...request.requestKey(buildId), params],
    queryFn: () => request.request(buildId, params),
    enabled: Boolean(buildId)
  })
}

export const fetchHTTPLog = (buildId: string, params?: RequestParams) => {
  return orionApiClient.builds.getBuildsLogsV2().request(buildId, params)
}
