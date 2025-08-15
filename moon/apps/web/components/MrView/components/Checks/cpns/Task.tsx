import { useState } from 'react'
import { CheckIcon, ChevronDownIcon, ChevronRightIcon, XIcon } from '@primer/octicons-react'
import { useAtom } from 'jotai'

import { LoadingSpinner } from '@gitmono/ui/Spinner'

import { buildId } from '@/components/Issues/utils/store'
import { TaskResult } from '@/hooks/SSE/useGetMrTask'

import { loadingAtom, Status, statusAtom } from './store'

export const mocks = [
  {
    arguments: '--env=prod --force',
    build_id: 'BUILD_20250813001',
    end_at: '2025-08-13T16:20:00Z',
    exit_code: 0,
    mr: 'MR-125',
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
    mr: 'MR-126',
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
    mr: 'MR-127',
    output_file: 'output_build_20250813003.zip',
    repo_name: 'data-processor',
    start_at: '2025-08-13T18:10:00Z',
    target: 'testing'
  }
]

export const Task = ({ list }: { list: TaskResult[] }) => {
  const [extend, setExtend] = useState(false)
  const [_, setBuildId] = useAtom(buildId)
  const [_loading, setLoading] = useAtom(loadingAtom)
  const [status] = useAtom(statusAtom)

  list = mocks

  const handleClick = (build_id: string) => {
    // 此处建立连接
    setLoading(true)
    setBuildId(build_id)
    // if (eventSourcesRef.current[build_id]) return
    // setEventSource(build_id)
  }

  const identifyStatus = (status: string) => {
    switch (status) {
      case Status.Success:
        return <CheckIcon size={14} className='text-[#1a7f37]' />
      case Status.Fail:
        return <XIcon size={14} className='text-[#d53d46]' />

      default:
        return <LoadingSpinner />
    }
  }

  return (
    <>
      <div
        onClick={() => setExtend(!extend)}
        className='flex w-full cursor-pointer items-center gap-4 border border-t-0 bg-[#fff] pl-4'
      >
        {extend ? <ChevronRightIcon size={16} /> : <ChevronDownIcon size={16} />}
        <div className='flex flex-col justify-center'>
          <span className='font-weight fz-[14px] text-[#1f2328]'>Task</span>
          <span className='fz-[12px] font-light text-[#59636e]'>side title</span>
        </div>
        {/* {extend && list} */}
      </div>
      {!extend && list && (
        <div className='fz-[14px] border-b pl-4 font-medium text-[#0969da]'>
          {list.map((i) => (
            <div
              onClick={() => handleClick(i.build_id)}
              className='!fz-[14px] flex !h-[37px] items-center gap-2'
              key={i.build_id}
            >
              {identifyStatus(status[i.build_id])}
              <span className='cursor-pointer hover:text-[#1f2328]'>{i.mr}</span>
            </div>
          ))}
        </div>
      )}
    </>
  )
}
