import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'

interface TestAPIResponse {
	req_result: boolean;
	data: Data[];
	err_message: string;
}

interface Data {
	oid: string;
	name: string;
	content_type: string;
	message: string;
	date: string;
}

export function useTestInfo(path: string) {
  return useQuery({
    queryKey: ['path', path],
    queryFn: () => 
      fetch(`${legacyApiClient.baseUrl}/api/v1/tree/commit-info?path=${path}`)
        .then(res => res.json() as Promise<TestAPIResponse>)
  })
} 