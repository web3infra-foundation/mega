import { useQuery } from "@tanstack/react-query";
import { legacyApiClient } from "@/utils/queryClient";

export function useGetMrFileTree(link: string) {
  return useQuery({
    queryKey: legacyApiClient.v1.getApiMrMuiTree().requestKey(link),
    queryFn: () => legacyApiClient.v1.getApiMrMuiTree().request(link),
    enabled: !!link,
  })
}