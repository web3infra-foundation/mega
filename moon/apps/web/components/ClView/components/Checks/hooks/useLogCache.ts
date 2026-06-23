import { useCallback, useEffect, useMemo, useRef } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'

import { GetBuildsLogsV2Data } from '@gitmono/types/generated'

import { getBuildLogQueryKey, getBuildLogQueryOptions, isTerminalBuildStatus } from '@/hooks/SSE/useGetHTTPLog'

import { useTaskSSE } from '../../../hook/useSSM'
import { getQueuedBuildIds, TaskInfoDTO } from '../cpns/store'
import { getTasksSignature } from '../cpns/tasksSignature'
import {
  findBuildInTasks,
  getAdjacentBuildIds,
  LogStatus,
  parseBuildLogResponse,
  TERMINAL_BUILD_STATUSES
} from './logUtils'

export function useLogCache(cl: string, buildId: string, tasks: TaskInfoDTO[] | undefined) {
  const queryClient = useQueryClient()
  const { eventSourcesRef, setEventSource, closeEventSource, logsMap, setLogsMap } = useTaskSSE(cl)
  const tasksSignatureRef = useRef('')

  const queuedBuildIds = useMemo(() => (tasks ? getQueuedBuildIds(tasks) : new Set<string>()), [tasks])

  const currentBuild = useMemo(
    () => (tasks && buildId ? findBuildInTasks(tasks, buildId) : undefined),
    [tasks, buildId]
  )

  const isQueued = Boolean(buildId && queuedBuildIds.has(buildId))
  const isBuilding = currentBuild?.status === 'Building'
  const isTerminal = isTerminalBuildStatus(currentBuild?.status)

  const httpEnabled = Boolean(buildId) && !isQueued && !isBuilding

  const {
    data: httpLog,
    isLoading: isHttpLoading,
    isError: isHttpError,
    isFetching: isHttpFetching,
    refetch: refetchHttpLog
  } = useQuery({
    ...getBuildLogQueryOptions(buildId, isTerminal),
    enabled: httpEnabled
  })

  // Manage SSE connections only when the tasks signature changes.
  useEffect(() => {
    if (!tasks?.length) return

    const signature = getTasksSignature(tasks)

    if (signature === tasksSignatureRef.current) return

    tasksSignatureRef.current = signature

    const buildingIds = new Set<string>()

    tasks.forEach((task) => {
      task.build_list?.forEach((build) => {
        if (build.status === 'Building' && !queuedBuildIds.has(build.id)) {
          buildingIds.add(build.id)
        }
      })
    })

    buildingIds.forEach((id) => {
      setEventSource(id)
    })

    Object.keys(eventSourcesRef.current).forEach((id) => {
      if (!buildingIds.has(id)) {
        closeEventSource(id)
      }
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tasks, queuedBuildIds])

  // Merge HTTP log into logsMap (never replace longer SSE snapshot).
  useEffect(() => {
    if (!httpEnabled || !buildId || !httpLog) return

    const { text } = parseBuildLogResponse(httpLog)

    if (!text) return

    setLogsMap((prev) => {
      const current = prev[buildId] ?? ''
      const next = current.length > text.length ? current : text

      if (prev[buildId] === next) return prev

      return { ...prev, [buildId]: next }
    })
  }, [httpEnabled, buildId, httpLog, setLogsMap])

  // Prefetch adjacent terminal builds on idle.
  useEffect(() => {
    if (!buildId || !tasks?.length) return

    const adjacent = getAdjacentBuildIds(tasks, buildId)

    adjacent.forEach((id) => {
      const build = findBuildInTasks(tasks, id)

      if (!build || build.status === 'Building' || queuedBuildIds.has(id)) return

      const terminal = TERMINAL_BUILD_STATUSES.has(build.status)

      void queryClient.prefetchQuery(getBuildLogQueryOptions(id, terminal))
    })
  }, [buildId, tasks, queryClient, queuedBuildIds])

  // Hydrate logsMap from react-query cache (e.g. prefetched adjacent builds).
  useEffect(() => {
    if (!tasks?.length) return

    const updates: Record<string, string> = {}

    tasks.forEach((task) => {
      task.build_list?.forEach((build) => {
        const cached = queryClient.getQueryData<GetBuildsLogsV2Data>(getBuildLogQueryKey(build.id))

        if (!cached) return

        const { text } = parseBuildLogResponse(cached)

        if (text) updates[build.id] = text
      })
    })

    if (Object.keys(updates).length === 0) return

    setLogsMap((prev) => {
      let changed = false
      const next = { ...prev }

      Object.entries(updates).forEach(([id, text]) => {
        if (!next[id]) {
          next[id] = text
          changed = true
        }
      })

      return changed ? next : prev
    })
  }, [tasks, queryClient, setLogsMap])

  const getLogStatus = useCallback(
    (id: string): LogStatus => {
      if (queuedBuildIds.has(id)) return 'idle'

      if (logsMap[id]) return 'success'

      if (id !== buildId) {
        const cached = queryClient.getQueryData<GetBuildsLogsV2Data>(getBuildLogQueryKey(id))

        if (cached) {
          const { status } = parseBuildLogResponse(cached)

          return status
        }

        return 'idle'
      }

      if (isBuilding) {
        return logsMap[id] ? 'success' : 'loading'
      }

      if (isHttpError) return 'error'

      if (isHttpLoading || isHttpFetching) return 'loading'

      if (httpLog) return parseBuildLogResponse(httpLog).status

      return 'idle'
    },
    [buildId, httpLog, isBuilding, isHttpError, isHttpFetching, isHttpLoading, logsMap, queryClient, queuedBuildIds]
  )

  const currentLogStatus = buildId ? getLogStatus(buildId) : 'idle'

  const logsAvailableIds = useMemo(() => {
    const ids = new Set<string>()

    Object.entries(logsMap).forEach(([id, text]) => {
      if (text) ids.add(id)
    })

    return ids
  }, [logsMap])

  const retryLog = useCallback(() => {
    if (!buildId) return

    void queryClient.invalidateQueries({ queryKey: getBuildLogQueryKey(buildId) })
    void refetchHttpLog()
  }, [buildId, queryClient, refetchHttpLog])

  return {
    logsMap,
    logsAvailableIds,
    getLogStatus,
    currentLogStatus,
    isQueued,
    isBuilding,
    retryLog
  }
}
