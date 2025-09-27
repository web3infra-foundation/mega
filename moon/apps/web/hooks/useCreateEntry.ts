import { apiClient } from "@/utils/queryClient";
import { CreateEntryInfo } from "@gitmono/types/generated";
import { useMutation } from "@tanstack/react-query";

export function useCreateEntry(){
    return useMutation({
        mutationFn: (data: CreateEntryInfo) => {
            return apiClient.v1.postApiCreateEntry().request(data)
        }
    });
}