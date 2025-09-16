import { useQuery } from '@tanstack/react-query'

import { GetTasksByMrData } from '@gitmono/types/generated'

import { fetchTask } from './ssmRequest'

export interface Tasks {
  build_list: BuildList[]
  created_at: string
  mr_id: number
  task_id: string
  task_name: string
  template: string
}

export interface BuildList {
  args: string[]
  created_at: string
  end_at: string
  exit_code: number
  id: string
  output_file: string
  repo: string
  start_at: string
  status: string
  target: string
  task_id: string
}

export function useGetMrTask(mr: number) {
  return useQuery<GetTasksByMrData, Error>({
    queryKey: [mr],
    queryFn: () => fetchTask(mr)
    // refetchInterval: 15000,
    // refetchIntervalInBackground: true,
    // enabled: !!mr
  })
}
