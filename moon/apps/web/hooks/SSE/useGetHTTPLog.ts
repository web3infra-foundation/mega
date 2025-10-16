import { useQuery } from '@tanstack/react-query'

import { GetTaskHistoryOutputByIdData, GetTaskHistoryOutputByIdParams } from '@gitmono/types/generated'

import { HttpTaskRes } from './ssmRequest'

export function useGetHTTPLog(payload: GetTaskHistoryOutputByIdParams) {
  return useQuery<GetTaskHistoryOutputByIdData, Error>({
    queryKey: [payload.id, payload],
    queryFn: () => HttpTaskRes(payload)
    // refetchInterval: 15000,
    // refetchIntervalInBackground: true,
    // enabled: !!cl
  })
}
