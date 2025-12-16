import React from 'react'

import { CommonResultVecMuiTreeNode } from '@gitmono/types'

import FileDiff from '@/components/DiffView/FileDiff'
import { useGetClFileChanged } from '@/hooks/CL/useGetClFileChanged'
import { useGetClFileTree } from '@/hooks/CL/useGetClFileTree'

interface FileChangeTabProps {
  id: string
}

export const FileChangeTab = React.memo<FileChangeTabProps>(({ id }) => {
  const PER_PAGE = 100

  const [page, setPage] = React.useState(1)
  const [isScrollLoading, setIsScrollLoading] = React.useState(false)

  const { fileChanged: ClFilesChangedData, isLoading: isFileChangeLoading } = useGetClFileChanged(id, {
    page,
    per_page: PER_PAGE
  })

  const { data: treeResponse, isLoading: treeIsLoading } = useGetClFileTree(id)

  const totalPages = ClFilesChangedData?.total ?? 0
  const hasMoreData = totalPages > 0 && page < totalPages

  React.useEffect(() => {
    if (!isFileChangeLoading && isScrollLoading) {
      setIsScrollLoading(false)
    }
  }, [isFileChangeLoading, isScrollLoading])

  const isInitialLoading =
    (isFileChangeLoading || treeIsLoading) &&
    page === 1 &&
    (!ClFilesChangedData?.items || ClFilesChangedData.items.length === 0)

  const isBlockingLoading = isInitialLoading || isScrollLoading

  const handleLoadMore = React.useCallback(() => {
    if (isScrollLoading || isFileChangeLoading || !hasMoreData) return
    setIsScrollLoading(true)
    setPage((prev) => prev + 1)
  }, [hasMoreData, isFileChangeLoading, isScrollLoading])

  return (
    <FileDiff
      fileChangeData={ClFilesChangedData}
      fileChangeIsLoading={isFileChangeLoading}
      treeData={treeResponse?.data as CommonResultVecMuiTreeNode['data']}
      treeIsLoading={treeIsLoading}
      page={page}
      hasMoreData={hasMoreData}
      isBlockingLoading={isBlockingLoading}
      onLoadMore={handleLoadMore}
    />
  )
})

FileChangeTab.displayName = 'FileChangeTab'
