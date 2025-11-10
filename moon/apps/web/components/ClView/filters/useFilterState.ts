import { useCallback, useState } from 'react'

export interface FilterState {
  author: string
  assignees: string[]
  labels: string[]
  review?: string
  order?: string
}

interface UseFilterStateOptions {
  scope: string
  type: 'issue' | 'cl'
  initialFilters?: Partial<FilterState>
}

export function useFilterState({ type, initialFilters = {} }: UseFilterStateOptions) {
  const [author, setAuthor] = useState<string>(initialFilters.author || '')
  const [assignees, setAssignees] = useState<string[]>(initialFilters.assignees || [])
  const [labels, setLabels] = useState<string[]>(initialFilters.labels || [])
  const [review, setReview] = useState<string>(initialFilters.review || '')
  // const [ order, setOrder ] = useState<string>(initialFilters.order || '')

  const [lastFilterState, setLastFilterState] = useState<string>('')

  const clearAllFilters = useCallback(() => {
    setAuthor('')
    setAssignees([])
    setLabels([])
    if (type === 'cl') {
      setReview('')
    }
    // 清空时重置上次状态
    setLastFilterState('')
  }, [type])

  const toApiParams = useCallback(() => {
    const params: {
      author?: string
      assignees?: string[]
      labels?: number[]
      review?: string
    } = {}

    if (author) params.author = author
    if (assignees.length > 0) params.assignees = assignees
    if (labels.length > 0) params.labels = labels.map(Number)

    if (type === 'cl' && review) params.review = review

    return params
  }, [author, assignees, labels, review, type])

  const toQueryString = useCallback(
    (labelsData: { id: number | string; name: string }[]) => {
      const parts: string[] = []

      if (author) parts.push(`author:${author}`)

      if (assignees.length > 0) {
        assignees.forEach((assignee) => parts.push(`assignee:${assignee}`))
      }

      if (labels.length > 0) {
        const labelNames = labels
          .map((id) => labelsData.find((l) => String(l.id) === id)?.name)
          .filter((name): name is string => Boolean(name))

        labelNames.forEach((name) => parts.push(`label:${name}`))
      }

      if (type === 'cl' && review) parts.push(`review:${review}`)

      return parts.join(' ')
    },
    [author, assignees, labels, review, type]
  )

  // 检查筛选条件是否真的改变了
  const hasChanged = useCallback(() => {
    const currentState = JSON.stringify({ author, assignees, labels, review })

    if (currentState === lastFilterState) {
      return false
    }
    // 更新上次状态
    setLastFilterState(currentState)
    return true
  }, [author, assignees, labels, review, lastFilterState])

  return {
    filters: {
      author,
      assignees,
      labels,
      ...(type === 'cl' ? { review } : {})
    },

    setAuthor,
    setAssignees,
    setLabels,
    setReview,

    toApiParams,
    toQueryString,
    hasChanged,

    clearAllFilters
  }
}
