import { useQuery } from '@tanstack/react-query'

import { taskStatus } from './ssmRequest'

export interface TaskStatus {
  exit_code: number
  message: string
  status: string
}

export function useGetTaskStatus(taskId: string) {
  return useQuery<TaskStatus, Error>({
    queryKey: [taskId],
    queryFn: () => taskStatus(taskId)
  })
}
