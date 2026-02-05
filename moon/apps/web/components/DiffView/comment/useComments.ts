import { useCallback, useMemo, useState } from 'react'
import type { AnnotationSide, DiffLineAnnotation, SelectedLineRange } from '@pierre/diffs'

import type { CodeReviewResponse } from '@gitmono/types/generated'

export function useComments(filePath: string, apiData?: CodeReviewResponse) {
  const [selectedRange, setSelectedRange] = useState<SelectedLineRange | null>(null)
  const [activeCommentLine, setActiveCommentLine] = useState<{
    side: AnnotationSide
    lineNumber: number
    filePath: string
  } | null>(null)

  const threads = useMemo(() => {
    return apiData?.files?.find((f) => f.file_path === filePath)?.threads || []
  }, [apiData, filePath])

  const annotations = useMemo((): DiffLineAnnotation<any>[] => {
    const result: DiffLineAnnotation<any>[] = []

    threads.forEach((thread) => {
      result.push({
        side: thread.anchor.diff_side === 'Deletions' ? 'deletions' : 'additions',
        lineNumber: Math.abs(thread.position.line_number),
        metadata: thread
      })
    })

    if (activeCommentLine?.filePath === filePath) {
      result.push({
        side: activeCommentLine.side,
        lineNumber: activeCommentLine.lineNumber,
        metadata: null
      })
    }

    return result
  }, [threads, activeCommentLine, filePath])

  const handleLineSelectionEnd = useCallback(
    (range: SelectedLineRange | null) => {
      setSelectedRange(range)
      if (!range) return

      const side: AnnotationSide = (range.endSide ?? range.side) === 'deletions' ? 'deletions' : 'additions'
      const lineNumber = Math.max(range.end, range.start)

      const hasExistingComment = threads.some(
        (thread) =>
          thread.position.line_number === lineNumber &&
          (thread.anchor.diff_side === 'Deletions' ? 'deletions' : 'additions') === side
      )

      if (!hasExistingComment) {
        setActiveCommentLine({ side, lineNumber, filePath })
      }
    },
    [filePath, threads]
  )

  const addCommentAtLine = useCallback(
    (side: AnnotationSide, lineNumber: number) => {
      const hasExistingComment = threads.some(
        (thread) =>
          Math.abs(thread.position.line_number) === lineNumber &&
          (thread.anchor.diff_side === 'Deletions' ? 'deletions' : 'additions') === side
      )

      if (!hasExistingComment) {
        setActiveCommentLine({ side, lineNumber, filePath })
      }
    },
    [filePath, threads]
  )

  const handleSubmitComment = useCallback(() => {
    setActiveCommentLine(null)
    setSelectedRange(null)
  }, [])

  const handleCancelComment = useCallback(() => {
    setActiveCommentLine(null)
    setSelectedRange(null)
  }, [])

  return {
    annotations,
    selectedRange,
    handleLineSelectionEnd,
    addCommentAtLine,
    handleSubmitComment,
    handleCancelComment
  }
}
