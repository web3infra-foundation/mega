import { useState } from 'react'
import { CheckIcon, ChevronDownIcon, ChevronRightIcon, XIcon } from '@primer/octicons-react'
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

export const mocks = [
  {
    arguments: '--env=prod --force',
    build_id: '0198b32b-6ede-7be2-99dc-aee8c7ef358d',
    end_at: '2025-08-13T16:20:00Z',
    exit_code: 0,
    cl: 'CL-125',
    output_file: 'output_build_20250813001.zip',
    repo_name: 'frontend-webapp',
    start_at: '2025-08-13T16:15:00Z',
    target: 'production'
  },
  {
    arguments: '--env=dev --skip-tests',
    build_id: 'BUILD_20250813002',
    end_at: '2025-08-13T17:05:00Z',
    exit_code: 1,
    cl: 'CL-126',
    output_file: 'output_build_20250813002.zip',
    repo_name: 'backend-service',
    start_at: '2025-08-13T16:50:00Z',
    target: 'development'
  },
  {
    arguments: '--env=test',
    build_id: 'BUILD_20250813003',
    end_at: '2025-08-13T18:30:00Z',
    exit_code: 0,
    cl: 'CL-127',
    output_file: 'output_build_20250813003.zip',
    repo_name: 'data-processor',
    start_at: '2025-08-13T18:10:00Z',
    target: 'testing'
  }
]

export const Task = ({ list }: { list: TaskInfoDTO }) => {
  const [extend, setExtend] = useState(false)

  // list = mocks

  return (
    <>
      <div
        onClick={() => setExtend(!extend)}
        className='flex w-full cursor-pointer items-center gap-4 border border-t-0 bg-[#fff] pl-4'
      >
        {extend ? <ChevronRightIcon size={16} /> : <ChevronDownIcon size={16} />}
        <div className='flex flex-col justify-center'>
          <span className='font-weight fz-[14px] text-[#1f2328]'>{list.task_name}</span>
          <span className='fz-[12px] font-light text-[#59636e]'>{formatDateTime(list.created_at)}</span>
        </div>
        {/* {extend && list} */}
      </div>
      {!extend && list && (
        <div className='fz-[14px] border-b pl-4 font-medium text-[#0969da]'>
          {list.build_list.map((i) => (
            <TaskItem key={i.id} build={i} />
          ))}
        </div>
      )}
    </>
  )
}

export const TaskItem = ({ build }: { build: BuildDTO }) => {
  // const [statusMap] = useAtom(statusMapAtom)

  const [_, setBuildId] = useAtom(buildIdAtom)
  const handleClick = (build_id: string) => {
    // 此处建立连接
    // setLoading(true)
    setBuildId(build_id)
    // if (eventSourcesRef.current[build_id]) return
    // setEventSource(build_id)
  }

  return (
    <>
      <div
        onClick={() => handleClick(build.id)}
        className='!fz-[14px] flex !h-[37px] items-center gap-2'
        key={build.id}
      >
        {/* {identifyStatus(statusMap.get(build.id)?.status || Status.NotFound)} */}
        {identifyStatus(build.status || Status.NotFound)}
        <span className='cursor-pointer hover:text-[#1f2328]'>{build.id}</span>
      </div>
    </>
  )
}

export const identifyStatus = (status: Status[keyof Status]) => {
  switch (status) {
    case Status.Completed:
      return <CheckIcon size={14} className='text-[#1a7f37]' />
    case Status.Failed:
      return <XIcon size={14} className='text-[#d53d46]' />
    case Status.Building:
      return <LoadingSpinner />
    case Status.Pending:
      return <LoadingSpinner />
    case Status.NotFound:
      return <LoadingSpinner />

    default:
      return <XIcon size={14} className='text-[#d53d46]' />
  }
}
