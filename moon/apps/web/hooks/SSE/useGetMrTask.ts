import { useQuery } from '@tanstack/react-query'

import { fetchTask } from './useRequest'

export interface TaskResult {
  arguments: string
  build_id: string
  end_at: string
  exit_code: number
  mr: string
  output_file: string
  repo_name: string
  start_at: string
  target: string
}

export function useGetMrTask(mr: string) {
  return useQuery<TaskResult[], Error>({
    queryKey: [mr],
    queryFn: () => fetchTask(mr)
  })
}
