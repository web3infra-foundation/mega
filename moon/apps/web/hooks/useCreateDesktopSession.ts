import { useMutation } from '@tanstack/react-query'

import { InternalDesktopSessionPostRequest } from '@gitmono/types'

import { apiClient } from '@/utils/queryClient'

export function useCreateDesktopSession() {
  return useMutation({
    mutationFn: (data: InternalDesktopSessionPostRequest) => apiClient.signIn.postSignInDesktop().request(data)
  })
}
