import { useQuery } from '@tanstack/react-query'

import { Status } from '@/components/ClView/components/Checks/cpns/store'

import { ClTaskStatus } from './ssmRequest'

export interface CLTaskStatus {
  arguments: string
  build_id: string
  end_at: string
  exit_code: number
  cl: string
  output_file: string
  repo_name: string
  start_at: string
  status: Status
  target: string
}

export function useGetClTaskStatus(cl: string) {
  return useQuery<CLTaskStatus[], Error>({
    queryKey: [cl],
    queryFn: () => ClTaskStatus(cl),
    refetchInterval: 15000,
    refetchIntervalInBackground: true,
    enabled: !!cl
  })
}
