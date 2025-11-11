import React from 'react'

import FileDiff from '@/components/DiffView/FileDiff'

interface FileChangeTabProps {
  id: string
}

export const FileChangeTab = React.memo<FileChangeTabProps>(({ id }) => <FileDiff id={id} />)

FileChangeTab.displayName = 'FileChangeTab'
