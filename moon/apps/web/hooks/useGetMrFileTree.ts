import { useQuery } from "@tanstack/react-query";
import { legacyApiClient } from "@/utils/queryClient";

export function useGetMrFileTree(link: string,path?: string) {
  return useQuery({
    queryKey: legacyApiClient.v1.getApiMrMuiTree().requestKey({ link, path }),
    queryFn: () => legacyApiClient.v1.getApiMrMuiTree().request({ link, path }),
    enabled: !!link,
  })
}