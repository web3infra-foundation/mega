import { useQuery } from '@tanstack/react-query'

import { GetTaskBuildListByIdData } from '@gitmono/types/generated'

import { fetchAllbuildList } from './ssmRequest'

export function useGetHTTPLog(id: string) {
  return useQuery<GetTaskBuildListByIdData, Error>({
    queryKey: [id],
    queryFn: () => fetchAllbuildList(id)
    // refetchInterval: 15000,
    // refetchIntervalInBackground: true,
    // enabled: !!cl
  })
}
