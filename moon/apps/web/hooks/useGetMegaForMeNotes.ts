import { keepPreviousData, useInfiniteQuery } from '@tanstack/react-query'

import { GetMembersMeForMeNotesParams } from '@gitmono/types'

import { apiClient } from '@/utils/queryClient'

interface Props {
  enabled?: boolean
  order?: GetMembersMeForMeNotesParams['order']
  query?: GetMembersMeForMeNotesParams['q']
}

const getMembersMeForMeNotes = apiClient.organizations.getMembersMeForMeNotes()

export function useGetMegaForMeNotes({
  enabled = true,
  order = { by: 'created_at', direction: 'desc' },
  query
}: Props) {
  return useInfiniteQuery({
    queryKey: getMembersMeForMeNotes.requestKey({ orgSlug: 'mega', order, q: query }),
    queryFn: ({ pageParam }) => getMembersMeForMeNotes.request({ orgSlug: 'mega', after: pageParam, order, q: query }),
    getNextPageParam: (lastPage) => lastPage.next_cursor,
    getPreviousPageParam: (firstPage) => firstPage.prev_cursor,
    initialPageParam: undefined as string | undefined,
    enabled,
    refetchOnWindowFocus: true,
    placeholderData: keepPreviousData
  })
}
