import { memo, useEffect, useState } from 'react'
import { LazyLog } from '@melloware/react-logviewer'
import { useAtom } from 'jotai'

import { LoadingSpinner } from '@gitmono/ui'

import { buildIdAtom } from '@/components/Issues/utils/store'
import { useGetClTask } from '@/hooks/SSE/useGetClTask'
import { fetchHTTPLog } from '@/hooks/SSE/useGetHTTPLog'

import { useTaskSSE } from '../../hook/useSSM'
import { statusMapAtom } from './cpns/store'
import { Task } from './cpns/Task'

type LogStatus = 'idle' | 'loading' | 'success' | 'empty' | 'error'

const Checks = ({ cl }: { cl: number }) => {
  const [buildid, setBuildId] = useAtom(buildIdAtom)
  const { logsMap, setEventSource, eventSourcesRef, setLogsMap } = useTaskSSE()
  const [statusMap, _setStatusMap] = useAtom(statusMapAtom)
  const { data: tasks, isError: isTasksError, isLoading: isTasksLoading } = useGetClTask(cl)
  const [logStatus, setLogStatus] = useState<Record<string, LogStatus>>({})

  useEffect(() => {
    if (!tasks || tasks.length === 0) return

    const builds = tasks.flatMap((task) =>
      (task.build_list ?? []).map((build) => ({
        build_id: build.id,
        task_id: build.task_id ?? task.task_id,
        repo: build.repo
      }))
    )

    const validBuilds = builds.filter((b) => Boolean(b.build_id && b.task_id && b.repo))

    if (validBuilds.length === 0) return

    validBuilds.forEach((b) => {
      setEventSource(b.build_id)
      setLogStatus((prev) => ({ ...prev, [b.build_id]: 'loading' }))
    })

    const fetchLogs = async () => {
      const logsResult = await Promise.allSettled(
        validBuilds.map(async ({ build_id, task_id, repo }) => {
          try {
            const res = await fetchHTTPLog({ task_id, build_id, repo })

            return { id: build_id, res, error: null }
          } catch (error) {
            return { id: build_id, res: null, error }
          }
        })
      )

      const newLogsMap: Record<string, string> = {}
      const newLogStatus: Record<string, LogStatus> = {}

      logsResult.forEach((item) => {
        if (item.status === 'fulfilled' && item.value) {
          const { id, res, error } = item.value

          if (error) {
            // fetchHTTPLog threw an error
            newLogStatus[id] = 'error'
            newLogsMap[id] = ''
          } else if (!res || !res.data) {
            // Response is null/undefined
            newLogStatus[id] = 'empty'
            newLogsMap[id] = ''
          } else if (Array.isArray(res.data) && res.data.length === 0) {
            // Response data is empty array
            newLogStatus[id] = 'empty'
            newLogsMap[id] = ''
          } else if (res.len === 0) {
            // Response len is 0
            newLogStatus[id] = 'empty'
            newLogsMap[id] = ''
          } else {
            // Success case
            const logText = Array.isArray(res.data) ? res.data.join('\n') : String(res.data || '')

            newLogStatus[id] = 'success'
            newLogsMap[id] = logText
          }
        } else {
          // Promise.allSettled rejected (shouldn't happen with try-catch, but defensive)
          const id = validBuilds[logsResult.indexOf(item)]?.build_id

          if (id) {
            newLogStatus[id] = 'error'
            newLogsMap[id] = ''
          }
        }
      })

      setLogsMap(newLogsMap)
      setLogStatus((prev) => ({ ...prev, ...newLogStatus }))
    }

    fetchLogs()

    if (!buildid && validBuilds.length > 0) {
      setBuildId(validBuilds[0].build_id)
    }

    return () => {
      statusMap.clear()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tasks])

  // Handle tasks loading state
  if (isTasksLoading) {
    return (
      <div className='bg-[#f6f8fa]' style={{ height: `calc(100vh - 104px)` }}>
        <div className='flex h-[60px] items-center border-b bg-white px-4'>
          <span>
            <h2 className='text-bold fz-[14px] text-[#59636e]'>[] tasks status interface</h2>
          </span>
        </div>
        <div className='flex h-full items-center justify-center text-gray-500'>
          <div className='flex items-center gap-3'>
            <LoadingSpinner />
            <span>Loading tasks...</span>
          </div>
        </div>
      </div>
    )
  }

  // Handle tasks error or empty state
  if (isTasksError) {
    return (
      <div className='bg-[#f6f8fa]' style={{ height: `calc(100vh - 104px)` }}>
        <div className='flex h-[60px] items-center border-b bg-white px-4'>
          <span>
            <h2 className='text-bold fz-[14px] text-[#59636e]'>[] tasks status interface</h2>
          </span>
        </div>
        <div className='flex h-full items-center justify-center text-red-500'>
          <span>Failed to fetch tasks</span>
        </div>
      </div>
    )
  }

  const validTasks = tasks?.filter((t) => t.build_list && t.build_list.length > 0) || []

  if (!isTasksLoading && (!tasks || tasks.length === 0 || validTasks.length === 0)) {
    return (
      <div className='bg-[#f6f8fa]' style={{ height: `calc(100vh - 104px)` }}>
        <div className='flex h-[60px] items-center border-b bg-white px-4'>
          <span>
            <h2 className='text-bold fz-[14px] text-[#59636e]'>[] tasks status interface</h2>
          </span>
        </div>
        <div className='flex h-full items-center justify-center text-gray-500'>
          <span>No tasks available</span>
        </div>
      </div>
    )
  }

  // Render log viewer with status handling
  const renderLogContent = () => {
    if (!buildid) {
      return (
        <div className='flex h-full items-center justify-center text-gray-500'>
          <span>Select a build to view logs</span>
        </div>
      )
    }

    const status = logStatus[buildid]

    // If status is undefined or idle, user needs to select a build
    if (!status || status === 'idle') {
      return (
        <div className='flex h-full items-center justify-center text-gray-500'>
          <span>Select a build to view logs</span>
        </div>
      )
    }

    if (status === 'loading') {
      return (
        <div className='flex h-full items-center justify-center text-gray-500'>
          <span>Loading logs...</span>
        </div>
      )
    }

    if (status === 'error') {
      return (
        <div className='flex h-full items-center justify-center text-red-500'>
          <span>Failed to fetch logs</span>
        </div>
      )
    }

    if (status === 'empty') {
      return (
        <div className='flex h-full items-center justify-center text-gray-500'>
          <span>No logs available</span>
        </div>
      )
    }

    if (status === 'success' && logsMap[buildid] && eventSourcesRef.current[buildid]) {
      return (
        <div className='h-full select-text [&_*]:select-text'>
          <LazyLog extraLines={1} text={logsMap[buildid]} stream enableSearch caseInsensitive follow />
        </div>
      )
    }

    // Fallback: show select prompt
    return (
      <div className='flex h-full items-center justify-center text-gray-500'>
        <span>Select a build to view logs</span>
      </div>
    )
  }

  return (
    <>
      <div className='bg-[#f6f8fa]' style={{ height: `calc(100vh - 104px)` }}>
        <div className='flex h-[60px] items-center border-b bg-white px-4'>
          <span>
            <h2 className='text-bold fz-[14px] text-[#59636e]'>[] tasks status interface</h2>
          </span>
        </div>
        <div className='flex justify-between' style={{ height: `calc(100vh - 164px)` }}>
          <div className='h-full w-[40%] overflow-y-auto border-r'>
            {validTasks.map((t) => (
              <Task key={t.task_id} list={t} logStatus={logStatus} />
            ))}
          </div>
          <div className='flex-1'>{renderLogContent()}</div>
        </div>
      </div>
    </>
  )
}

export default memo(Checks)
