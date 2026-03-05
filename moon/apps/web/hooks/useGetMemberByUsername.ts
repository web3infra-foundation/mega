import { useQuery } from '@tanstack/react-query'

import type { GetMembersByUsernameData, RequestParams } from '@gitmono/types'

import { useScope } from '@/contexts/scope'
import { apiClient } from '@/utils/queryClient'

const getMembersByUsername = apiClient.organizations.getMembersByUsername()

export function useGetMemberByUsername(username: string, params?: RequestParams) {
  const { scope } = useScope()

  return useQuery<GetMembersByUsernameData, Error>({
    queryKey: [...getMembersByUsername.requestKey(`${scope}`, username), params],
    queryFn: () => getMembersByUsername.request(`${scope}`, username, params),
    enabled: !!scope && !!username,
    staleTime: 5 * 60 * 1000, // 5 minutes
    retry: false
  })
}
