import { useMutation, useQueryClient } from '@tanstack/react-query'
import toast from 'react-hot-toast'

import type { CreateGroupRequest, PostApiAdminGroupsData } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useCreateAdminGroup() {
  const queryClient = useQueryClient()

  return useMutation<PostApiAdminGroupsData, Error, CreateGroupRequest>({
    mutationFn: (data: CreateGroupRequest) => legacyApiClient.v1.postApiAdminGroups().request(data),
    onSuccess: () => {
      // Refresh user groups list
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.postApiAdminGroupsList().baseKey
      })
      toast.success('Group created successfully')
    }
    // Remove onError: apiErrorToast, let component handle errors itself
  })
}
