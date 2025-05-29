import { NewIssue, PostApiIssueNewData } from '@gitmono/types/generated'
import { useMutation } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostIssueSubmit() {
  return useMutation<PostApiIssueNewData, Error, { data: NewIssue }>({
    mutationFn: ({ data }) => legacyApiClient.v1.postApiIssueNew().request(data)
  })
}
