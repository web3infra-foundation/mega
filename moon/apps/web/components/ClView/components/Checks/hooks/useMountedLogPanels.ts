import { useEffect, useState } from 'react'

import { MAX_MOUNTED_LOG_PANELS } from './logUtils'

/** LRU list of build ids with mounted LogViewer panels. */
export function useMountedLogPanels(buildId: string, logsMap: Record<string, string>) {
  const [mountedIds, setMountedIds] = useState<string[]>([])

  useEffect(() => {
    if (!buildId || !logsMap[buildId]) return

    setMountedIds((prev) => {
      const withLogs = prev.filter((id) => logsMap[id] && id !== buildId)
      const next = [...withLogs, buildId]

      while (next.length > MAX_MOUNTED_LOG_PANELS) {
        next.shift()
      }

      return next
    })
  }, [buildId, logsMap])

  return mountedIds.filter((id) => logsMap[id])
}
