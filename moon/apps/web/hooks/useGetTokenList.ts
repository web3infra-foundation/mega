import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useAtom } from 'jotai'
import { atomFamily } from 'jotai/utils'

import { ListToken } from '@gitmono/types'

import { atomWithWebStorage } from '@/utils/atomWithWebStorage'
import { legacyApiClient } from '@/utils/queryClient'

const fetchTokenList = legacyApiClient.v1.getApiUserTokenList()
const getTokenListAtom = atomFamily(() => atomWithWebStorage<ListToken[]>(`token`, []))

export const useGetTokenList = () => {
  const [tokenList, setTokenList] = useAtom(getTokenListAtom('token'))

  const { data, isLoading, isPending, isFetching } = useQuery({
    queryKey: fetchTokenList.requestKey(),
    queryFn: async () => {
      const result = await fetchTokenList.request()

      return result.data
    }
  })

  useEffect(() => {
    if (data) {
      setTokenList(data)
    }
  }, [data, setTokenList])

  return {
    tokenList,
    isLoading,
    isPending,
    isFetching
  }
}
