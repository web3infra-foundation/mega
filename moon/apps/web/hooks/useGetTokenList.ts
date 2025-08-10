import { atomFamily } from 'jotai/utils'
import { atomWithWebStorage } from '@/utils/atomWithWebStorage'
import { legacyApiClient } from '@/utils/queryClient'
import { useQuery } from '@tanstack/react-query'
import { useAtom } from 'jotai'
import { useMemo } from 'react'
import { ListToken } from '@gitmono/types'

const fetchTokenList = legacyApiClient.v1.getApiUserTokenList()
const getTokenListAtom = atomFamily(() =>
  atomWithWebStorage<ListToken[]>(`token`, [])
)

export const useGetTokenList = () => {
  const [, setTokenList] = useAtom(getTokenListAtom())

  const {data, isLoading, isPending, isFetching} = useQuery({
    queryKey: fetchTokenList.requestKey(),
    queryFn: async () => {
      const result = await fetchTokenList.request()

      return result.data
    },
  })

  const tokenList = useMemo(() => {
    setTokenList(data)
    return data ?? []
  }, [data, setTokenList])

  return {
    tokenList,
    isLoading,
    isPending,
    isFetching
  }
}
