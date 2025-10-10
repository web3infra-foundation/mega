import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'

const query = legacyApiClient.v1.getApiTreeCommitInfo()

export function useGetTreeCommitInfo(path: string) {

	return useQuery({
		queryKey: query.requestKey({path}),
		queryFn: () => query.request({path})
	})
} 