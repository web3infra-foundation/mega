import {legacyApiClient} from "@/utils/queryClient";
import {atomFamily} from "jotai/utils";
import {atomWithWebStorage} from "@/utils/atomWithWebStorage";
import {ListSSHKey} from "@gitmono/types";
import {useAtom} from "jotai";
import { useQuery } from '@tanstack/react-query'
import {useEffect} from 'react'

const fetchSSHList = legacyApiClient.v1.getApiUserSshList()
const getSSHListAtom = atomFamily(() =>
  atomWithWebStorage<ListSSHKey[]>(`ssh-key`, [])
)

export const useGetSSHList = () => {
  const [sshKeys, setSSHList] = useAtom(getSSHListAtom('ssh-key'))

  const { data, isLoading, isPending, isFetching } = useQuery({
    queryKey: fetchSSHList.requestKey(),
    queryFn: async () => {
      const result = await fetchSSHList.request()

      return result.data
    },
  });

  useEffect(() => {
    if(data){
      setSSHList(data)
    }
  }, [data, setSSHList]);

  return { sshKeys, isLoading, isPending, isFetching }
}