import { useMutation } from '@tanstack/react-query'
import { PostApiUserTokenGenerateData } from '@gitmono/types'
import { legacyApiClient } from '@/utils/queryClient'

export function usePostTokenGenerate() {
  return useMutation<PostApiUserTokenGenerateData, Error>({
    mutationFn: () => legacyApiClient.v1.postApiUserTokenGenerate().request()
  })
}