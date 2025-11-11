import { useMutation } from '@tanstack/react-query'

import { NewGpgRequest, PostApiGpgAddData } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export const usePostGPGKey = () => {
  return useMutation<PostApiGpgAddData, Error, { data: NewGpgRequest }>({
    mutationFn: ({ data }) => legacyApiClient.v1.postApiGpgAdd().request(data)
  })
}
