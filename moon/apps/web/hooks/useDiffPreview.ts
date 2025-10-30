import { useMutation } from '@tanstack/react-query'

import { DiffPreviewPayload } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useDiffPreview() {
  return useMutation({
    mutationFn: (data: DiffPreviewPayload) => {
      return legacyApiClient.v1.postApiEditDiffPreview().request(data)
    }
  })
}
