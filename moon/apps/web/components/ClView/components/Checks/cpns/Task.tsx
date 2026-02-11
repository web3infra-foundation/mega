import { useState } from 'react'
import { CheckIcon, ChevronDownIcon, FileDirectoryIcon, XIcon } from '@primer/octicons-react'
import { format } from 'date-fns'
import { useAtom } from 'jotai'

import { BuildDTO, TaskInfoDTO } from '@gitmono/types/generated'
import { LoadingSpinner } from '@gitmono/ui/Spinner'

import { buildIdAtom } from '@/components/Issues/utils/store'

import { Status } from './store'

/**
 * Format ISO date string to readable format
 * @param isoDate - ISO 8601 date string (e.g., "2025-11-03T13:16:36.477167+00:00")
 * @returns Formatted date string (e.g., "2025-11-04 11:22")
 */
const formatDateTime = (isoDate: string): string => {
  try {
    return format(new Date(isoDate), 'yyyy-MM-dd HH:mm')
  } catch {
    return isoDate
  }
}

type LogStatus = 'idle' | 'loading' | 'success' | 'empty' | 'error'

export interface TreeRootProps {
  path?: string
  tasks: TaskInfoDTO[]
  logStatus: Record<string, LogStatus>
  totalTasksCount?: number
}

/**
 * Tree Root Component - Top level node showing the path
 */
export const TreeRoot = ({ path, tasks, logStatus, totalTasksCount }: TreeRootProps) => {
  const [isExpanded, setIsExpanded] = useState(true)

  // Show the total number of tasks in dropdown (displayed as "builds")
  const displayCount = totalTasksCount ?? tasks.length

  return (
    <div className='select-none'>
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className='group flex w-full cursor-pointer items-center gap-2.5 border-b border-gray-200 bg-gradient-to-r from-blue-50 to-transparent px-3 py-3 transition-all hover:from-blue-100 hover:to-blue-50 dark:border-gray-700 dark:from-blue-900/20 dark:hover:from-blue-900/30 dark:hover:to-blue-900/10'
      >
        <div className='flex items-center gap-2'>
          <div
            className='transition-transform duration-200'
            style={{ transform: isExpanded ? 'rotate(0deg)' : 'rotate(-90deg)' }}
          >
            <ChevronDownIcon size={16} className='text-gray-600 dark:text-gray-400' />
          </div>
          <FileDirectoryIcon size={16} className='text-blue-600 dark:text-blue-400' />
        </div>
        <span className='text-sm font-semibold text-gray-800 dark:text-gray-200'>{path || 'Project Root'}</span>
        <span className='ml-auto rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'>
          {displayCount} {displayCount === 1 ? 'build' : 'builds'}
        </span>
      </div>
      {isExpanded && (
        <div className='bg-gray-50/50 dark:bg-gray-900/20'>
          {tasks.map((t, index) => (
            <Task key={t.task_id} list={t} logStatus={logStatus} isLast={index === tasks.length - 1} />
          ))}
        </div>
      )}
    </div>
  )
}

/**
 * Task Component - Second level showing task name and builds
 */
export const Task = ({
  list,
  logStatus,
  isLast
}: {
  list: TaskInfoDTO
  logStatus: Record<string, LogStatus>
  isLast?: boolean
}) => {
  const [isExpanded, setIsExpanded] = useState(true)

  // Extract filename from output_file path (from first target's first build)
  const getFileName = () => {
    if (!list.targets || list.targets.length === 0) return list.task_name || 'Unnamed Task'

    const firstTarget = list.targets[0] as any

    if (!firstTarget.builds || firstTarget.builds.length === 0) return list.task_name || 'Unnamed Task'

    const firstBuild = firstTarget.builds[0]

    if (!firstBuild.output_file) return list.task_name || 'Unnamed Task'

    const parts = firstBuild.output_file.split('/')

    return parts[parts.length - 1] || 'Unnamed Task'
  }

  // Get overall task status from targets with styled badge
  const getTaskStatus = () => {
    if (!list.targets || list.targets.length === 0) return null

    const states = list.targets.map((t) => t.state)

    // If any target failed, show failed
    if (states.some((s) => s === 'Failed')) {
      return {
        status: 'Failed',
        icon: <XIcon size={14} className='text-red-600 dark:text-red-400' />,
        badgeClass: 'bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300'
      }
    }

    // If any target interrupted, show interrupted
    if (states.some((s) => s === 'Interrupted')) {
      return {
        status: 'Interrupted',
        icon: <XIcon size={14} className='text-orange-600 dark:text-orange-400' />,
        badgeClass: 'bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-300'
      }
    }

    // If any target is building, show building
    if (states.some((s) => s === 'Building')) {
      return {
        status: 'Building',
        icon: <LoadingSpinner />,
        badgeClass: 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'
      }
    }

    // If any target is pending, show pending
    if (states.some((s) => s === 'Pending')) {
      return {
        status: 'Pending',
        icon: <LoadingSpinner />,
        badgeClass: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/40 dark:text-yellow-300'
      }
    }

    // If all completed, show completed
    if (states.every((s) => s === 'Completed')) {
      return {
        status: 'Completed',
        icon: <CheckIcon size={14} className='text-green-700 dark:text-green-400' />,
        badgeClass: 'bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-300'
      }
    }

    return null
  }

  const taskStatus = getTaskStatus()
  const fileName = getFileName()

  return (
    <div className={`relative ${!isLast ? 'border-b border-gray-200 dark:border-gray-700' : ''}`}>
      {/* Vertical line connector */}
      <div className='absolute left-6 top-0 h-full w-px bg-gray-300 dark:bg-gray-600' />

      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className='group relative flex w-full cursor-pointer items-center gap-3 bg-white px-3 py-2.5 transition-all hover:bg-gray-100 dark:bg-gray-800/50 dark:hover:bg-gray-700/50'
      >
        {/* Horizontal line connector */}
        <div className='absolute left-6 top-1/2 h-px w-3 bg-gray-300 dark:bg-gray-600' />

        <div className='relative z-10 ml-6 flex items-center gap-2'>
          <div
            className='transition-transform duration-200'
            style={{ transform: isExpanded ? 'rotate(0deg)' : 'rotate(-90deg)' }}
          >
            <ChevronDownIcon size={14} className='text-gray-500 dark:text-gray-400' />
          </div>
        </div>

        <div className='flex flex-1 flex-col gap-0.5'>
          <span className='text-sm font-medium text-gray-800 dark:text-gray-200'>{fileName}</span>
          <span className='text-xs text-gray-500 dark:text-gray-400'>{formatDateTime(list.created_at)}</span>
        </div>

        {taskStatus && (
          <div className='flex items-center gap-2'>
            {taskStatus.icon}
            <span className={`rounded-md px-2 py-0.5 text-xs font-medium ${taskStatus.badgeClass}`}>
              {taskStatus.status}
            </span>
          </div>
        )}
      </div>

      {isExpanded && list && (
        <div className='bg-gray-50 dark:bg-gray-900/30'>
          {list.build_list.map((i, index) => (
            <TaskItem key={i.id} build={i} logStatus={logStatus[i.id]} isLast={index === list.build_list.length - 1} />
          ))}
        </div>
      )}
    </div>
  )
}

/**
 * TaskItem Component - Build item showing individual builds
 */
export const TaskItem = ({
  build,
  logStatus,
  isLast
}: {
  build: BuildDTO
  logStatus?: LogStatus
  isLast?: boolean
}) => {
  const [buildId, setBuildId] = useAtom(buildIdAtom)

  const handleClick = (build_id: string) => {
    setBuildId(build_id)
  }

  const isSelected = buildId === build.id
  const isHighlighted = logStatus === 'success'

  let bgClass = 'bg-white dark:bg-gray-800/30'
  let textColor = 'text-gray-600 dark:text-gray-400'
  let borderColor = 'border-gray-300 dark:border-gray-600'

  if (isSelected) {
    bgClass = 'bg-blue-50 dark:bg-blue-900/20'
    textColor = 'text-blue-700 dark:text-blue-300'
    borderColor = 'border-blue-400 dark:border-blue-500'
  } else if (isHighlighted) {
    textColor = 'text-blue-600 dark:text-blue-400'
  }

  return (
    <div className='relative'>
      {/* Vertical line connector */}
      <div className={`absolute left-6 top-0 ${isLast ? 'h-1/2' : 'h-full'} w-px bg-gray-300 dark:bg-gray-600`} />

      <div
        onClick={() => handleClick(build.id)}
        className={`group relative flex h-10 cursor-pointer items-center gap-3 px-3 transition-all hover:bg-gray-100 dark:hover:bg-gray-700/30 ${bgClass}`}
      >
        {/* Horizontal line connector */}
        <div className={`absolute left-6 top-1/2 h-px w-6 ${borderColor}`} />

        {/* Status dot */}
        <div
          className={`relative z-10 ml-12 flex h-2 w-2 items-center justify-center rounded-full ${borderColor} border-2 bg-white dark:bg-gray-800`}
        >
          {isSelected && <div className='h-1 w-1 rounded-full bg-blue-500' />}
        </div>

        <div className='flex items-center gap-2'>
          {identifyStatus(build.status || Status.NotFound)}
          <span
            className={`font-mono text-sm transition-colors ${textColor} group-hover:text-blue-600 dark:group-hover:text-blue-400`}
          >
            {build.id}
          </span>
        </div>

        {isSelected && (
          <div className='ml-auto'>
            <div className='h-1.5 w-1.5 animate-pulse rounded-full bg-blue-500' />
          </div>
        )}
      </div>
    </div>
  )
}

export const identifyStatus = (status: Status[keyof Status]) => {
  switch (status) {
    case Status.Completed:
      return <CheckIcon size={14} className='text-green-700 dark:text-green-400' />
    case Status.Failed:
      return <XIcon size={14} className='text-red-600 dark:text-red-400' />
    case Status.Building:
      return <LoadingSpinner />
    case Status.Pending:
      return <LoadingSpinner />
    case Status.NotFound:
      return <LoadingSpinner />

    default:
      return <XIcon size={14} className='text-red-600 dark:text-red-400' />
  }
}
