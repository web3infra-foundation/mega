import { useQuery } from '@tanstack/react-query'

import { Status } from '@/components/MrView/components/Checks/cpns/store'

import { MrTaskStatus } from './ssmRequest'

export interface MRTaskStatus {
  arguments: string
  build_id: string
  end_at: string
  exit_code: number
  mr: string
  output_file: string
  repo_name: string
  start_at: string
  status: Status
  target: string
}

export function useGetMrTaskStatus(mr: string) {
  return useQuery<MRTaskStatus[], Error>({
    queryKey: [mr],
    queryFn: () => MrTaskStatus(mr),
    refetchInterval: 15000,
    refetchIntervalInBackground: true,
    enabled: !!mr
  })
}
