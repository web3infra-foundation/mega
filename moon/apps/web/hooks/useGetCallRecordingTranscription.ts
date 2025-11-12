import { useQuery } from '@tanstack/react-query'

import { useScope } from '@/contexts/scope'
import { apiClient } from '@/utils/queryClient'

const query = apiClient.organizations.getCallRecordingsTranscription()

interface Props {
  callRecordingId: string
}

export function useGetCallRecordingTranscription({ callRecordingId }: Props) {
  const { scope } = useScope()

  return useQuery({
    queryKey: query.requestKey(`${scope}`, callRecordingId),
    queryFn: () => query.request(`${scope}`, callRecordingId)
  })
}
