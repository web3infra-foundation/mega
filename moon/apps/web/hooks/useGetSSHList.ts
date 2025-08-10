import {legacyApiClient} from "@/utils/queryClient";
import {atomFamily} from "jotai/utils";
import {atomWithWebStorage} from "@/utils/atomWithWebStorage";
import {ListSSHKey} from "@gitmono/types";
import {useAtom} from "jotai";
import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'

const fetchSSHList = legacyApiClient.v1.getApiUserSshList()
const getSSHListAtom = atomFamily(() =>
  atomWithWebStorage<ListSSHKey[]>(`ssh-key`, [])
)

export const useGetSSHList = () => {
  const [, setSSHList] = useAtom(getSSHListAtom())

  const { data, isLoading, isPending, isFetching } = useQuery({
    queryKey: fetchSSHList.requestKey(),
    queryFn: async () => {
      const result = await fetchSSHList.request()

      return result.data
    },
  });

  const sshKeys = useMemo(() => {
    setSSHList(data);
    return data ?? []
  }, [data, setSSHList]);

  return { sshKeys, isLoading, isPending, isFetching }
}