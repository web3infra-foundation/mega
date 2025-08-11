import {useMutation} from "@tanstack/react-query";
import {GetApiLabelByIdData} from "@gitmono/types";
import {legacyApiClient} from "@/utils/queryClient";

export function useGetLabelById() {
  return useMutation<GetApiLabelByIdData, Error, { id: number }>({
    mutationFn: ({ id }) => legacyApiClient.v1.getApiLabelById().request(id)
  })
}