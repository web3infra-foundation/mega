import { useMutation, useQueryClient } from '@tanstack/react-query'

import { UsersMeTwoFactorAuthenticationDeleteRequest } from '@gitmono/types'

import { apiClient } from '@/utils/queryClient'

export function useDisableTwoFactorAuthentication() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (data: UsersMeTwoFactorAuthenticationDeleteRequest) =>
      apiClient.users.deleteMeTwoFactorAuthentication().request(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: apiClient.users.getMe().requestKey() })
    }
  })
}
