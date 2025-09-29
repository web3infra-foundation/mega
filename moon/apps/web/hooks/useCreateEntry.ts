import { legacyApiClient } from "@/utils/queryClient";
import { CreateEntryInfo } from "@gitmono/types/generated";
import { useMutation } from "@tanstack/react-query";

export function useCreateEntry(){
    return useMutation({
        mutationFn: (data: CreateEntryInfo) => {
            return legacyApiClient.v1.postApiCreateEntry().request(data)
        }
    });
}