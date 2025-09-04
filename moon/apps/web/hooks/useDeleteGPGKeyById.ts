import { useMutation } from "@tanstack/react-query";
import { DeleteApiGpgRemoveData, RemoveGpgRequest } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const useDeleteGPGKeyById = () => {
  return useMutation<DeleteApiGpgRemoveData, Error, { data: RemoveGpgRequest }>({
    mutationFn: ({ data }) => legacyApiClient.v1.deleteApiGpgRemove().request(data)
  })
};