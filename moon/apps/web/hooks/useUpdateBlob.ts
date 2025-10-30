import { useMutation, useQueryClient } from '@tanstack/react-query'

import { EditFilePayload } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useUpdateBlob() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (data: EditFilePayload) => {
      return legacyApiClient.v1.postApiEditSave().request(data)
    },
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiBlob().baseKey
        }),
        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiTree().baseKey
        }),
        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiTreeCommitInfo().baseKey
        }),
        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiBlame().baseKey
        })
      ])
    }
  })
}
