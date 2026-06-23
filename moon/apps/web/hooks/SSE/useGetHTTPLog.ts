import { useQuery } from '@tanstack/react-query'

import { GetBuildsLogsV2Data, RequestParams } from '@gitmono/types/generated'

import { BuildStatus } from '@/components/ClView/components/Checks/cpns/store'
import { TERMINAL_BUILD_STATUSES } from '@/components/ClView/components/Checks/hooks/logUtils'
import { orionApiClient } from '@/utils/queryClient'

const request = orionApiClient.builds.getBuildsLogsV2()

export function getBuildLogQueryKey(buildId: string, params?: RequestParams) {
  return [...request.requestKey(buildId), params] as const
}

export function getBuildLogQueryOptions(buildId: string, isTerminal: boolean, params?: RequestParams) {
  return {
    queryKey: getBuildLogQueryKey(buildId, params),
    queryFn: () => request.request(buildId, params),
    enabled: Boolean(buildId),
    staleTime: isTerminal ? Number.POSITIVE_INFINITY : 0,
    gcTime: 1000 * 60 * 30
  }
}

export function useGetHTTPLog(buildId: string, isTerminal: boolean, params?: RequestParams) {
  return useQuery<GetBuildsLogsV2Data, Error>(getBuildLogQueryOptions(buildId, isTerminal, params))
}

export const fetchHTTPLog = (buildId: string, params?: RequestParams) => {
  return request.request(buildId, params)
}

export function isTerminalBuildStatus(status: BuildStatus | undefined): boolean {
  return Boolean(status && TERMINAL_BUILD_STATUSES.has(status))
}
