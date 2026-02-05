import { memo, useCallback, useEffect, useRef, useState } from 'react'
import { LazyLog } from '@melloware/react-logviewer'
import { useAtom } from 'jotai'

import { LoadingSpinner } from '@gitmono/ui'

import { buildIdAtom } from '@/components/Issues/utils/store'
import { useGetClTask } from '@/hooks/SSE/useGetClTask'
import { fetchHTTPLog } from '@/hooks/SSE/useGetHTTPLog'

import { useTaskSSE } from '../../hook/useSSM'
import { statusMapAtom } from './cpns/store'
import { TreeRoot } from './cpns/Task'

type LogStatus = 'idle' | 'loading' | 'success' | 'empty' | 'error'

const MIN_LEFT_WIDTH = 200
const MAX_LEFT_WIDTH_PERCENT = 0.7
const DEFAULT_LEFT_WIDTH_PERCENT = 0.3

const Checks = ({ cl, path }: { cl: number; path?: string }) => {
  const [buildid, setBuildId] = useAtom(buildIdAtom)
  const { logsMap, setEventSource, eventSourcesRef, setLogsMap } = useTaskSSE()
  const [statusMap, _setStatusMap] = useAtom(statusMapAtom)
  const { data: tasks, isError: isTasksError, isLoading: isTasksLoading } = useGetClTask(cl)
  const [logStatus, setLogStatus] = useState<Record<string, LogStatus>>({})

  // Resizable panel state
  const containerRef = useRef<HTMLDivElement>(null)
  const leftPanelRef = useRef<HTMLDivElement>(null)
  const rightPanelRef = useRef<HTMLDivElement>(null)
  const [leftWidth, setLeftWidth] = useState<number | null>(null)
  const [isDragging, setIsDragging] = useState(false)
  const scrollPositionRef = useRef<number>(0)
  const logContainerRef = useRef<HTMLDivElement>(null)
  const startWidthRef = useRef<number>(0)

  // Initialize left width based on container width
  useEffect(() => {
    if (containerRef.current && leftWidth === null) {
      setLeftWidth(containerRef.current.offsetWidth * DEFAULT_LEFT_WIDTH_PERCENT)
    }
  }, [leftWidth])

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!containerRef.current || !leftPanelRef.current) return

    // Directly manipulate DOM without triggering React re-render
    const containerRect = containerRef.current.getBoundingClientRect()
    const newLeftWidth = e.clientX - containerRect.left
    const maxWidth = containerRect.width * MAX_LEFT_WIDTH_PERCENT
    const clampedWidth = Math.max(MIN_LEFT_WIDTH, Math.min(newLeftWidth, maxWidth))

    // Update DOM directly for smooth dragging
    leftPanelRef.current.style.width = `${clampedWidth}px`
  }, [])

  const handleMouseUp = useCallback(() => {
    // Remove event listeners
    document.removeEventListener('mousemove', handleMouseMove)
    document.removeEventListener('mouseup', handleMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''

    // Show right panel immediately using DOM
    if (rightPanelRef.current) {
      rightPanelRef.current.style.display = 'block'
    }

    // Update React state only once when dragging ends
    if (leftPanelRef.current) {
      const finalWidth = leftPanelRef.current.offsetWidth

      setLeftWidth(finalWidth)
    }
    setIsDragging(false)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()

      // Immediately hide right panel using DOM (no re-render)
      if (rightPanelRef.current) {
        rightPanelRef.current.style.display = 'none'
      }

      // Save scroll position
      if (logContainerRef.current) {
        const lazyLogElement = logContainerRef.current.querySelector('.react-lazylog')

        if (lazyLogElement) {
          scrollPositionRef.current = lazyLogElement.scrollTop
        }
      }

      // Save current width
      if (leftPanelRef.current) {
        startWidthRef.current = leftPanelRef.current.offsetWidth
      }

      // Update state asynchronously (won't block dragging)
      requestAnimationFrame(() => {
        setIsDragging(true)
      })

      // Add event listeners immediately
      document.addEventListener('mousemove', handleMouseMove)
      document.addEventListener('mouseup', handleMouseUp)
      document.body.style.cursor = 'col-resize'
      document.body.style.userSelect = 'none'
    },
    [handleMouseMove, handleMouseUp]
  )

  useEffect(() => {
    if (!isDragging) {
      // Restore scroll position after dragging ends
      if (logContainerRef.current && scrollPositionRef.current > 0) {
        // Use requestAnimationFrame to ensure DOM is updated
        requestAnimationFrame(() => {
          const lazyLogElement = logContainerRef.current?.querySelector('.react-lazylog')

          if (lazyLogElement) {
            lazyLogElement.scrollTop = scrollPositionRef.current
          }
        })
      }
    }
  }, [isDragging])

  // Reset scroll position when buildid changes
  useEffect(() => {
    // Clear saved scroll position when switching to a different build
    scrollPositionRef.current = 0

    // Reset scroll to top for new build
    if (logContainerRef.current) {
      requestAnimationFrame(() => {
        const lazyLogElement = logContainerRef.current?.querySelector('.react-lazylog')

        if (lazyLogElement) {
          lazyLogElement.scrollTop = 0
        }
      })
    }
  }, [buildid])

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
      <div className='bg-secondary' style={{ height: `calc(100vh - 104px)` }}>
        <div className='border-primary bg-primary flex h-[60px] items-center border-b px-4'>
          <span>
            <h2 className='text-tertiary text-bold fz-[14px]'>[] tasks status interface</h2>
          </span>
        </div>
        <div className='text-tertiary flex h-full items-center justify-center'>
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
      <div className='bg-secondary' style={{ height: `calc(100vh - 104px)` }}>
        <div className='border-primary bg-primary flex h-[60px] items-center border-b px-4'>
          <span>
            <h2 className='text-tertiary text-bold fz-[14px]'>[] tasks status interface</h2>
          </span>
        </div>
        <div className='flex h-full items-center justify-center text-red-500 dark:text-red-400'>
          <span>Failed to fetch tasks</span>
        </div>
      </div>
    )
  }

  const validTasks = tasks?.filter((t) => t.build_list && t.build_list.length > 0) || []

  if (!isTasksLoading && (!tasks || tasks.length === 0 || validTasks.length === 0)) {
    return (
      <div className='bg-secondary' style={{ height: `calc(100vh - 104px)` }}>
        <div className='border-primary bg-primary flex h-[60px] items-center border-b px-4'>
          <span>
            <h2 className='text-tertiary text-bold fz-[14px]'>[] tasks status interface</h2>
          </span>
        </div>
        <div className='text-tertiary flex h-full items-center justify-center'>
          <span>No tasks available</span>
        </div>
      </div>
    )
  }

  // Render log viewer with status handling
  const renderLogContent = () => {
    if (!buildid) {
      return (
        <div className='text-tertiary flex h-full items-center justify-center'>
          <span>Select a build to view logs</span>
        </div>
      )
    }

    const status = logStatus[buildid]

    // If status is undefined or idle, user needs to select a build
    if (!status || status === 'idle') {
      return (
        <div className='text-tertiary flex h-full items-center justify-center'>
          <span>Select a build to view logs</span>
        </div>
      )
    }

    if (status === 'loading') {
      return (
        <div className='text-tertiary flex h-full items-center justify-center'>
          <span>Loading logs...</span>
        </div>
      )
    }

    if (status === 'error') {
      return (
        <div className='flex h-full items-center justify-center text-red-500 dark:text-red-400'>
          <span>Failed to fetch logs</span>
        </div>
      )
    }

    if (status === 'empty') {
      return (
        <div className='text-tertiary flex h-full items-center justify-center'>
          <span>No logs available</span>
        </div>
      )
    }

    if (status === 'success' && logsMap[buildid] && eventSourcesRef.current[buildid]) {
      return (
        <div ref={logContainerRef} className='h-full select-text [&_*]:select-text'>
          <LazyLog key={buildid} extraLines={1} text={logsMap[buildid]} stream enableSearch caseInsensitive />
        </div>
      )
    }

    // Fallback: show select prompt
    return (
      <div className='text-tertiary flex h-full items-center justify-center'>
        <span>Select a build to view logs</span>
      </div>
    )
  }

  return (
    <>
      <div className='bg-secondary' style={{ height: `calc(100vh - 104px)` }}>
        <div className='border-primary bg-primary flex h-[60px] items-center border-b px-4'>
          <span>
            <h2 className='text-tertiary text-bold fz-[14px]'>[] tasks status interface</h2>
          </span>
        </div>
        <div ref={containerRef} className='flex' style={{ height: `calc(100vh - 164px)` }}>
          <div
            ref={leftPanelRef}
            className='border-primary h-full overflow-y-auto border-r'
            style={{ width: leftWidth ?? '30%', flexShrink: 0 }}
          >
            <TreeRoot path={path} tasks={validTasks} logStatus={logStatus} />
          </div>
          {/* Resizer handle */}
          <div
            onMouseDown={handleMouseDown}
            className='border-primary h-full w-1 flex-shrink-0 cursor-col-resize transition-colors hover:bg-blue-400'
            style={{ backgroundColor: isDragging ? '#60a5fa' : undefined }}
          />
          <div ref={rightPanelRef} className='flex-1' style={{ display: isDragging ? 'none' : 'block' }}>
            {renderLogContent()}
          </div>
          {isDragging && (
            <div className='text-tertiary flex flex-1 items-center justify-center'>
              <span>Resizing...</span>
            </div>
          )}
        </div>
      </div>
    </>
  )
}

export default memo(Checks)
