import { useMutation, useQueryClient } from '@tanstack/react-query'
import toast from 'react-hot-toast'

import { DeleteApiTagsByNameData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

const del = legacyApiClient.v1.deleteApiTagsByName()

export function useDeleteMonoTag(params?: RequestParams) {
  const qc = useQueryClient()

  return useMutation<DeleteApiTagsByNameData, Error, string>({
    mutationFn: (name) => del.request(name, params),
    onSuccess: (_, name) => {
      // invalidate item and list
      qc.invalidateQueries({ queryKey: del.requestKey(name) })
      qc.invalidateQueries({ queryKey: legacyApiClient.v1.postApiTagsList().baseKey })
      toast('Tag deleted')
    }
  })
}
