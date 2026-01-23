import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import { PostApiClUpdateBranchData } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

/**
 * Update CL branch
 * Merge the latest commits from the target branch into the current CL branch
 *
 * Business logic:
 * 1. Backend automatically identifies the target branch associated with the CL (e.g., main)
 * 2. Merges the latest commits from the target branch into the CL branch
 * 3. Updates the CL's base_commit to the latest commit of the target branch
 *
 * Use cases:
 * - When getApiClUpdateStatus returns outdated: true
 * - When user clicks the "Update Branch" button
 */
export const usePostClUpdateBranch = () => {
  const queryClient = useQueryClient()

  return useMutation<PostApiClUpdateBranchData, Error, string>({
    mutationFn: (link: string) => {
      const mutation = legacyApiClient.v1.postApiClUpdateBranch()

      return mutation.request(link)
    },
    onSuccess: (data, link) => {
      // Check the result
      if (data.req_result) {
        toast.success('Branch updated successfully')
      } else {
        toast.error(data.err_message || 'Failed to update branch')
      }

      // Invalidate related query caches
      // 1. Refresh update status
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClUpdateStatus().requestKey(link)
      })

      // 2. Refresh CL details
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClDetail().requestKey(link)
      })

      // 3. Refresh merge check status
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClMergeBox().requestKey(link)
      })
    },
    onError: (error: Error) => {
      toast.error(error.message || 'Failed to update branch, please try again later')
    }
  })
}
