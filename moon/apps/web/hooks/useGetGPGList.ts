import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useAtom } from 'jotai'
import { atomFamily } from 'jotai/utils'

import { GpgKey } from '@gitmono/types'

import { atomWithWebStorage } from '@/utils/atomWithWebStorage'
import { legacyApiClient } from '@/utils/queryClient'

const fetchGPGList = legacyApiClient.v1.getApiGpgList()
const getGPGListAtom = atomFamily(() => atomWithWebStorage<GpgKey[]>(`gpg-key`, []))

export const useGetGPGList = () => {
  const [gpgKeys, setGpgKeys] = useAtom(getGPGListAtom('gpg-key'))
  const { data, isLoading } = useQuery({
    queryKey: fetchGPGList.requestKey(),
    queryFn: async () => {
      const response = await fetchGPGList.request()

      return response.data
    }
  })

  useEffect(() => {
    if (data) {
      setGpgKeys(data)
    }
  }, [data, setGpgKeys])

  return { gpgKeys, isLoading }
}
