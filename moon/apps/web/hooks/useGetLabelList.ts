import { atomFamily } from 'jotai/utils'
import { atomWithWebStorage } from '@/utils/atomWithWebStorage'
import { LabelItem } from '@gitmono/types'
import { useAtom } from 'jotai'
import { useScope } from '@/contexts/scope'
import { useMemo } from 'react'
import { legacyApiClient } from '@/utils/queryClient'
import { useQuery } from '@tanstack/react-query'

const fetchLabelList = legacyApiClient.v1.postApiLabelList()
const getLabelListAtom = atomFamily((scope: string) =>
  atomWithWebStorage<LabelItem[]>(`${scope}:issue-label`, [])
)

export const useGetLabelList = () => {
  const { scope } = useScope();
  const [,setLabelList] = useAtom(getLabelListAtom(`${scope}`));

  const { data, isLoading, isPending, isFetching } = useQuery({
    queryKey: fetchLabelList.requestKey(),
    queryFn: async () => {
      const result = await fetchLabelList.request({
        additional: "",
        pagination: {
          page: 1,
          per_page: 100
        }
      })

      return result.data?.items?? []
    },
    enabled: !!scope
  });


  const labels = useMemo(() => {
    if (data) {
      setLabelList(data);
    }
    return data ?? []
  }, [data, setLabelList]);

  return { labels, isLoading, isPending, isFetching }
}