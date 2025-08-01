import { keepPreviousData, useQuery } from '@tanstack/react-query'

import { GetApiIssueIssueSuggesterParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetIssueIssueSuggester(query: GetApiIssueIssueSuggesterParams) {
  return useQuery({
    queryKey: legacyApiClient.v1.getApiIssueIssueSuggester().requestKey(query),
    queryFn: () => legacyApiClient.v1.getApiIssueIssueSuggester().request(query),
    placeholderData: keepPreviousData
  })
}
