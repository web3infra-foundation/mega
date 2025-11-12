import { useQuery } from '@tanstack/react-query'

import { GetApiLabelByIdData } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetLabelById(id: number) {
  return useQuery<GetApiLabelByIdData, Error>({
    queryKey: ['label', id],
    queryFn: () => legacyApiClient.v1.getApiLabelById().request(id)
  })
}
