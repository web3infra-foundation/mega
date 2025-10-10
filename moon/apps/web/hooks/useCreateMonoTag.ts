import { useMutation, useQueryClient } from '@tanstack/react-query'
import toast from 'react-hot-toast'

import { CreateTagRequest, PostApiTagsData, RequestParams } from '@gitmono/types'
import { legacyApiClient } from '@/utils/queryClient'

const createTag = legacyApiClient.v1.postApiTags()

export function useCreateMonoTag(params?: RequestParams) {
  const qc = useQueryClient()

  return useMutation<PostApiTagsData, Error, CreateTagRequest>({
    mutationFn: (data) => createTag.request(data, params),
    onSuccess: () => {
      // invalidate list queries
      qc.invalidateQueries({ queryKey: createTag.baseKey })
      qc.invalidateQueries({ queryKey: legacyApiClient.v1.postApiTagsList().baseKey })
      toast('Tag created')
    }
  })
}
