import { useQuery } from '@tanstack/react-query'

import { HttpTaskRes } from './ssmRequest'

export interface HTTPLogRes {
  task_id: string
  offset: number
  len: number
  data: string
  next_offset: number
  file_size: number
  eof: boolean
}

export function useGetHTTPLog(taskId: string) {
  return useQuery<HTTPLogRes, Error>({
    queryKey: [taskId],
    queryFn: () => HttpTaskRes(taskId, 0, 4096)
    // refetchInterval: 15000,
    // refetchIntervalInBackground: true,
    // enabled: !!mr
  })
}
