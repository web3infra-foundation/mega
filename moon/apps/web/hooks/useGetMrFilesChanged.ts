import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { RequestParams } from '@gitmono/types'
import type { GetApiMrFilesChangedData } from '@gitmono/types/generated'

export function useGetMrFilesChanged(id: string, params?: RequestParams) {
  return useQuery<GetApiMrFilesChangedData>({
    queryKey: legacyApiClient.v1.getApiMrFilesChanged().requestKey(id),
    queryFn: () => legacyApiClient.v1.getApiMrFilesChanged().request(id, params),
    enabled: !!id, 
  })
}