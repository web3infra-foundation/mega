import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { GetApiTreeCommitInfoParams, RequestParams } from '@gitmono/types'

const treeCommitInfoQuery = legacyApiClient.v1.getApiTreeCommitInfo()

export function useGetTreeCommitInfo(path: string, refs?: string, requestParams?: RequestParams) {
	const params: GetApiTreeCommitInfoParams = { path }
	
	if (refs) params.refs = refs

	return useQuery({
		// eslint-disable-next-line @tanstack/query/exhaustive-deps
		queryKey: treeCommitInfoQuery.requestKey(params),
		queryFn: () => treeCommitInfoQuery.request(params, requestParams)
	})
}