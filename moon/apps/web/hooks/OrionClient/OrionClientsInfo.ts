import { useMutation, useQueryClient } from '@tanstack/react-query'

import type { PageParamsOrionClientQuery, PostOrionClientsInfoData, RequestParams } from '@gitmono/types/generated'

import { orionApiClient } from '@/utils/queryClient'

export function usePostOrionClientsInfo(params?: RequestParams) {
  const queryClient = useQueryClient()
  const mutation = orionApiClient.postOrionClientsInfo()

  return useMutation<PostOrionClientsInfoData, Error, PageParamsOrionClientQuery>({
    mutationFn: (data) => mutation.request(data, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: mutation.baseKey
      })
    }
  })
}
