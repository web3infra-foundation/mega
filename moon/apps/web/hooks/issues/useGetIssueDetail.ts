import { useQuery } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

interface conversations {
  id: number
  user_id: number
  conv_type: string
  comment: string
  created_at: number
  updated_at: number
}

interface raw {
  id: number
  link: string
  title: string
  status: string
  open_timestamp: number
  conversations: conversations[]
}

interface issueDetail {
  status: string
  conversations: { id: number; user_id: number; conv_type: string; comment: string; created_at: number }[]
  title: string
}

interface detailRes {
  err_message: string
  data: issueDetail
  req_result: boolean
}
const getApiIssueDetail = legacyApiClient.v1.getApiIssueDetail()

export function useGetIssueDetail(id: string) {
  return useQuery<detailRes, Error>({
    queryKey: ['issueDetail', id],
    queryFn: async () => {
      const { err_message, data, req_result } = await getApiIssueDetail.request(id)

      if (!req_result) throw new Error(err_message || 'fetching failed')
      const rawData = data as unknown as raw
      const converted: issueDetail = {
        title: rawData.title,
        conversations: rawData.conversations,
        status: rawData.status
      }

      return {
        err_message,
        data: converted,
        req_result
      }
    },

    enabled: !!id
  })
}
