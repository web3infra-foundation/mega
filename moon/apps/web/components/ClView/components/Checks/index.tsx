import { memo, useCallback, useEffect, useRef, useState } from 'react'
import { LazyLog } from '@melloware/react-logviewer'
import { format } from 'date-fns'
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
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null)
  const [isDropdownOpen, setIsDropdownOpen] = useState(false)
  const [hoveredTaskId, setHoveredTaskId] = useState<string | null>(null)
  const [tooltipPosition, setTooltipPosition] = useState<{ top: number; left: number } | null>(null)

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

  // Initialize selected task
  useEffect(() => {
    const validTasks = tasks?.filter((t) => t.build_list && t.build_list.length > 0) || []

    if (validTasks.length > 0 && !selectedTaskId) {
      setSelectedTaskId(validTasks[0].task_id)
    }
  }, [tasks, selectedTaskId])

  // Helper functions for dropdown
  const getTaskFileName = (task: any) => {
    if (!task.targets || task.targets.length === 0) return task.task_name || 'Unnamed Task'

    const firstTarget = task.targets[0] as any

    if (!firstTarget.builds || firstTarget.builds.length === 0) return task.task_name || 'Unnamed Task'

    const firstBuild = firstTarget.builds[0]

    if (!firstBuild.output_file) return task.task_name || 'Unnamed Task'

    const parts = firstBuild.output_file.split('/')

    return parts[parts.length - 1] || 'Unnamed Task'
  }

  const formatDateTime = (isoDate: string): string => {
    try {
      return format(new Date(isoDate), 'yyyy-MM-dd HH:mm')
    } catch {
      return isoDate
    }
  }

  const getTaskStatus = (task: any) => {
    if (!task.targets || task.targets.length === 0) return null

    const states = task.targets.map((t: any) => t.state)

    if (states.some((s: string) => s === 'Failed')) {
      return { status: 'Failed', color: 'text-red-600 dark:text-red-400' }
    }

    if (states.some((s: string) => s === 'Interrupted')) {
      return { status: 'Interrupted', color: 'text-orange-600 dark:text-orange-400' }
    }

    if (states.some((s: string) => s === 'Building')) {
      return { status: 'Building', color: 'text-blue-600 dark:text-blue-400' }
    }

    if (states.some((s: string) => s === 'Pending')) {
      return { status: 'Pending', color: 'text-yellow-600 dark:text-yellow-400' }
    }

    if (states.every((s: string) => s === 'Completed')) {
      return { status: 'Completed', color: 'text-green-600 dark:text-green-400' }
    }

    return null
  }

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
  const selectedTask = validTasks.find((t) => t.task_id === selectedTaskId)
  const tasksToDisplay = selectedTask ? [selectedTask] : validTasks

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
        {/* Header with Task Selector */}
        <div className='border-primary bg-primary flex h-[60px] items-center border-b px-4'>
          <div className='flex flex-1 items-center gap-2'>
            <h2 className='text-tertiary text-bold fz-[14px]'>[] tasks status interface</h2>

            {/* Task Selector Dropdown - Only show if multiple tasks */}
            {validTasks.length > 1 && (
              <div className='relative' style={{ minWidth: '240px', maxWidth: '320px' }}>
                <button
                  onClick={(e) => {
                    e.stopPropagation()
                    setIsDropdownOpen(!isDropdownOpen)
                  }}
                  className='flex w-full items-center justify-between rounded-lg border border-gray-300 bg-white px-2.5 py-1.5 text-sm transition-all hover:border-blue-400 hover:bg-gray-50 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:border-gray-600 dark:bg-gray-800 dark:hover:border-blue-500 dark:hover:bg-gray-700'
                >
                  <div className='flex flex-1 items-center gap-2 overflow-hidden'>
                    {selectedTask ? (
                      <>
                        <span className='truncate font-medium text-gray-800 dark:text-gray-200'>
                          {getTaskFileName(selectedTask)}
                        </span>
                        {getTaskStatus(selectedTask) && (
                          <span className={`flex-shrink-0 text-xs ${getTaskStatus(selectedTask)?.color}`}>
                            • {getTaskStatus(selectedTask)?.status}
                          </span>
                        )}
                      </>
                    ) : (
                      <span className='text-gray-500 dark:text-gray-400'>Select a task...</span>
                    )}
                  </div>
                  <svg
                    className={`ml-2 h-4 w-4 flex-shrink-0 text-gray-500 transition-transform dark:text-gray-400 ${
                      isDropdownOpen ? 'rotate-180' : ''
                    }`}
                    fill='none'
                    stroke='currentColor'
                    viewBox='0 0 24 24'
                  >
                    <path strokeLinecap='round' strokeLinejoin='round' strokeWidth={2} d='M19 9l-7 7-7-7' />
                  </svg>
                </button>

                {/* Dropdown Menu */}
                {isDropdownOpen && (
                  <>
                    {/* Backdrop */}
                    <div className='fixed inset-0 z-10' onClick={() => setIsDropdownOpen(false)} />

                    {/* Dropdown List */}
                    <div className='absolute left-0 top-full z-20 mt-1 max-h-80 w-full overflow-y-auto rounded-lg border border-gray-300 bg-white shadow-lg dark:border-gray-600 dark:bg-gray-800'>
                      {validTasks.map((task) => {
                        const fileName = getTaskFileName(task)
                        const status = getTaskStatus(task)
                        const isSelected = task.task_id === selectedTaskId
                        const targetsCount = task.targets?.length || 0

                        return (
                          <div
                            key={task.task_id}
                            onClick={(e) => {
                              e.stopPropagation()
                              setSelectedTaskId(task.task_id)
                              setIsDropdownOpen(false)
                              setHoveredTaskId(null)
                            }}
                            onMouseEnter={(e) => {
                              setHoveredTaskId(task.task_id)

                              const rect = e.currentTarget.getBoundingClientRect()

                              setTooltipPosition({
                                top: rect.top,
                                left: rect.right + 8
                              })
                            }}
                            onMouseLeave={() => {
                              setHoveredTaskId(null)
                              setTooltipPosition(null)
                            }}
                            className={`flex cursor-pointer items-center justify-between px-3 py-2.5 transition-colors ${
                              isSelected ? 'bg-blue-50 dark:bg-blue-900/30' : 'hover:bg-gray-100 dark:hover:bg-gray-700'
                            }`}
                          >
                            <div className='flex flex-1 flex-col gap-1 overflow-hidden'>
                              <div className='flex items-center gap-2'>
                                <span
                                  className={`truncate text-sm font-medium ${
                                    isSelected ? 'text-blue-700 dark:text-blue-300' : 'text-gray-800 dark:text-gray-200'
                                  }`}
                                >
                                  {fileName}
                                </span>
                                {status && (
                                  <span className={`flex-shrink-0 text-xs ${status.color}`}>• {status.status}</span>
                                )}
                              </div>
                              <span className='truncate text-xs text-gray-500 dark:text-gray-400'>
                                {targetsCount} {targetsCount === 1 ? 'target' : 'targets'}
                              </span>
                            </div>
                            {isSelected && (
                              <svg
                                className='ml-2 h-4 w-4 flex-shrink-0 text-blue-600 dark:text-blue-400'
                                fill='currentColor'
                                viewBox='0 0 20 20'
                              >
                                <path
                                  fillRule='evenodd'
                                  d='M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z'
                                  clipRule='evenodd'
                                />
                              </svg>
                            )}
                          </div>
                        )
                      })}
                    </div>

                    {/* Tooltip for hovered task */}
                    {hoveredTaskId && tooltipPosition && (
                      <div
                        className='fixed z-30 max-w-md rounded-lg border border-gray-300 bg-white p-3 shadow-xl dark:border-gray-600 dark:bg-gray-800'
                        style={{
                          top: `${tooltipPosition.top}px`,
                          left: `${tooltipPosition.left}px`
                        }}
                      >
                        {(() => {
                          const task = validTasks.find((t) => t.task_id === hoveredTaskId)

                          if (!task) return null

                          const fileName = getTaskFileName(task)
                          const status = getTaskStatus(task)
                          const targetsCount = task.targets?.length || 0

                          return (
                            <div className='flex flex-col gap-2'>
                              <div className='flex items-center gap-2'>
                                <span className='font-semibold text-gray-800 dark:text-gray-200'>{fileName}</span>
                                {status && (
                                  <span
                                    className={`rounded-md px-2 py-0.5 text-xs font-medium ${
                                      status.status === 'Completed'
                                        ? 'bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-300'
                                        : status.status === 'Failed'
                                          ? 'bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300'
                                          : status.status === 'Building'
                                            ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'
                                            : status.status === 'Pending'
                                              ? 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/40 dark:text-yellow-300'
                                              : 'bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-300'
                                    }`}
                                  >
                                    {status.status}
                                  </span>
                                )}
                              </div>
                              <div className='text-xs text-gray-600 dark:text-gray-400'>
                                <div>Task ID: {task.task_id}</div>
                                <div>Created: {formatDateTime(task.created_at)}</div>
                                <div>Targets: {targetsCount}</div>
                              </div>
                            </div>
                          )
                        })()}
                      </div>
                    )}
                  </>
                )}
              </div>
            )}
          </div>
        </div>

        <div ref={containerRef} className='flex' style={{ height: `calc(100vh - 164px)` }}>
          <div
            ref={leftPanelRef}
            className='border-primary h-full overflow-y-auto border-r'
            style={{ width: leftWidth ?? '30%', flexShrink: 0 }}
          >
            <TreeRoot path={path} tasks={tasksToDisplay} logStatus={logStatus} totalTasksCount={validTasks.length} />
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
