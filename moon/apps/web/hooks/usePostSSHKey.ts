import { useMutation } from '@tanstack/react-query'

import { AddSSHKey, PostApiUserSshData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostSSHKey() {
  return useMutation<PostApiUserSshData, Error, { data: AddSSHKey; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiUserSsh().request(data, params)
  })
}
