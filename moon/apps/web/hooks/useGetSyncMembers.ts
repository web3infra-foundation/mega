import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'

import type { GetSyncMembersData } from '@gitmono/types'

import { useScope } from '@/contexts/scope'
import { useGetCurrentUser } from '@/hooks/useGetCurrentUser'
import { commandScoreSort } from '@/utils/commandScoreSort'
import { apiClient } from '@/utils/queryClient'

const getSyncMembers = apiClient.organizations.getSyncMembers()

interface Props {
  enabled?: boolean
  excludeCurrentUser?: boolean
  query?: string
}

export function useGetSyncMembers({ enabled = true, excludeCurrentUser = false, query = '' }: Props = {}) {
  const { scope } = useScope()
  const { data: currentUser, isLoading: isUserLoading } = useGetCurrentUser()

  // Only enable query when user is logged in and has scope
  const shouldFetch = !!scope && !!currentUser && enabled && !isUserLoading

  const {
    data: members = [],
    refetch,
    isLoading,
    isPending,
    isFetching,
    error
  } = useQuery<GetSyncMembersData>({
    queryKey: getSyncMembers.requestKey(`${scope}`),
    queryFn: () => getSyncMembers.request(`${scope}`),
    enabled: shouldFetch,
    staleTime: 5 * 60 * 1000, // 5 minutes
    retry: (failureCount, error: any) => {
      // Don't retry for permission errors
      if (error?.response?.status === 403 || error?.code === 'forbidden') {
        return false
      }
      return failureCount < 3
    }
  })

  const filtered = useMemo(() => {
    let temp = members

    if (excludeCurrentUser && currentUser?.id) {
      temp = temp.filter((member) => member.user.id !== currentUser.id)
    }

    // Filter out deactivated members
    temp = temp.filter((member) => !member.deactivated)

    // Apply search query
    if (query.trim()) {
      temp = commandScoreSort(temp, query, (member) => `${member.user.username} ${member.user.display_name}`)
    }

    return temp
  }, [members, excludeCurrentUser, currentUser?.id, query])

  return {
    members: filtered,
    allMembers: members,
    refetch,
    isLoading,
    isPending,
    isFetching,
    error,
    total: members.length
  }
}
