import React from 'react'

import { CommonResultVecMuiTreeNode } from '@gitmono/types'

import FileDiff from '@/components/DiffView/FileDiff'
import { useGetClFileChanged } from '@/hooks/CL/useGetClFileChanged'
import { useGetClFileTree } from '@/hooks/CL/useGetClFileTree'

interface FileChangeTabProps {
  id: string
}

export const FileChangeTab = React.memo<FileChangeTabProps>(({ id }) => {
  const [page] = React.useState(1)
  const { fileChanged: ClFilesChangedData, isLoading: isFileChangeLoading } = useGetClFileChanged(id, {
    page,
    per_page: 100
  })

  const { data: treeResponse, isLoading: treeIsLoading } = useGetClFileTree(id)

  return (
    <FileDiff
      fileChangeData={ClFilesChangedData}
      fileChangeIsLoading={isFileChangeLoading}
      treeData={treeResponse as CommonResultVecMuiTreeNode}
      treeIsLoading={treeIsLoading}
    />
  )
})

FileChangeTab.displayName = 'FileChangeTab'
