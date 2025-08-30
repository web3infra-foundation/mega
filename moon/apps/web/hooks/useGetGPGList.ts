import { legacyApiClient } from "@/utils/queryClient";
import { atomFamily } from "jotai/utils";
import { atomWithWebStorage } from "@/utils/atomWithWebStorage";
import { GpgKey } from "@gitmono/types";
import { useAtom } from "jotai";
import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";

const fetchGPGList = legacyApiClient.v1.getApiGpgList()
const getGPGListAtom = atomFamily(() =>
  atomWithWebStorage<GpgKey[]>(`gpg-key`, [])
)

export const useGetGPGList = () => {
  const [gpgKeys, setGpgKeys] = useAtom(getGPGListAtom('gpg-key'))
  const { data, isLoading } = useQuery({
    queryKey: fetchGPGList.requestKey(),
    queryFn: async () => {
      const response = await fetchGPGList.request()
      
      return response.data
    }
  });

  useEffect(() => {
    if(data) {
      setGpgKeys(data)
    }
  }, [data, setGpgKeys]);
  
  return { gpgKeys, isLoading }
};