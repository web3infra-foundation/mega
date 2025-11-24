import { useMutation } from '@tanstack/react-query'

import type { PageParamsListPayload, PostApiClListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

interface DraftClListVariables {
  data: Omit<PageParamsListPayload, 'additional'> & {
    additional?: Omit<PageParamsListPayload['additional'], 'status' | 'asc'>
  }
  params?: RequestParams
}

export function useDraftClList() {
  return useMutation<PostApiClListData, Error, DraftClListVariables>({
    mutationFn: ({ data, params }) => {
      const { additional, ...rest } = data

      const payload: PageParamsListPayload = {
        ...rest,
        additional: {
          ...(additional ?? {}),
          status: 'draft',
          asc: false
        }
      }

      return legacyApiClient.v1.postApiClList().request(payload, params)
    }
  })
}
