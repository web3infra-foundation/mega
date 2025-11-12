import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useAtom } from 'jotai'
import { atomFamily } from 'jotai/utils'

import { GetApiClMergeBoxData } from '@gitmono/types'

import { atomWithWebStorage } from '@/utils/atomWithWebStorage'
import { legacyApiClient } from '@/utils/queryClient'

const fetchMergeBox = legacyApiClient.v1.getApiClMergeBox()
const getMergeBoxAtom = atomFamily(() => atomWithWebStorage<GetApiClMergeBoxData['data']>(`merge-box`, {}))

export const useGetMergeBox = (link: string) => {
  const [mergeBoxData, setMergeBoxData] = useAtom(getMergeBoxAtom(`merge-box`))
  const { data, isLoading } = useQuery({
    queryKey: fetchMergeBox.requestKey(link),
    queryFn: async () => {
      const response = await fetchMergeBox.request(link)

      return response.data ?? {}
    }
  })

  useEffect(() => {
    if (data) {
      setMergeBoxData(data)
    }
  }, [data, setMergeBoxData])

  return { mergeBoxData, isLoading }
}
