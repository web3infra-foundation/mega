import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useAtom } from 'jotai'
import { atomFamily } from 'jotai/utils'

import { ListSSHKey } from '@gitmono/types'

import { atomWithWebStorage } from '@/utils/atomWithWebStorage'
import { legacyApiClient } from '@/utils/queryClient'

const fetchSSHList = legacyApiClient.v1.getApiUserSshList()
const getSSHListAtom = atomFamily(() => atomWithWebStorage<ListSSHKey[]>(`ssh-key`, []))

export const useGetSSHList = () => {
  const [sshKeys, setSSHList] = useAtom(getSSHListAtom('ssh-key'))

  const { data, isLoading, isPending, isFetching } = useQuery({
    queryKey: fetchSSHList.requestKey(),
    queryFn: async () => {
      const result = await fetchSSHList.request()

      return result.data
    }
  })

  useEffect(() => {
    if (data) {
      setSSHList(data)
    }
  }, [data, setSSHList])

  return { sshKeys, isLoading, isPending, isFetching }
}
