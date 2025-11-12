import { useMutation } from '@tanstack/react-query'

import { CreateEntryInfo } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useCreateEntry() {
  return useMutation({
    mutationFn: (data: CreateEntryInfo) => {
      return legacyApiClient.v1.postApiCreateEntry().request(data)
    }
  })
}
