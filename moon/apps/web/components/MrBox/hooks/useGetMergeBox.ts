import { useAtom } from "jotai";
import { atomFamily } from "jotai/utils";
import { atomWithWebStorage } from "@/utils/atomWithWebStorage";
import { useQuery } from "@tanstack/react-query";
import { legacyApiClient } from "@/utils/queryClient";
import { GetApiClMergeBoxData } from "@gitmono/types";
import { useEffect } from "react";

const fetchMergeBox = legacyApiClient.v1.getApiClMergeBox()
const getMergeBoxAtom = atomFamily(() =>
  atomWithWebStorage<GetApiClMergeBoxData['data']>(`merge-box`, {})
)

export const useGetMergeBox = (link: string) => {
  const [mergeBoxData, setMergeBoxData] = useAtom(getMergeBoxAtom(`merge-box`));
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
  }, [data, setMergeBoxData]);

  return { mergeBoxData, isLoading };
}