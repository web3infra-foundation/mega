import { memo, useEffect } from 'react'
import { LazyLog } from '@melloware/react-logviewer'
import { useAtom } from 'jotai'

import { LoadingSpinner } from '@gitmono/ui/Spinner'

import { buildId } from '@/components/Issues/utils/store'
import { TaskResult, useGetMrTask } from '@/hooks/SSE/useGetMrTask'
import { useGetMrTaskStatus } from '@/hooks/SSE/useGetMrTaskStatus'

import { useTaskSSE } from '../../hook/useSSM'
import { loadingAtom, statusMapAtom } from './cpns/store'
import { mocks, Task } from './cpns/Task'

const Checks = ({ mr }: { mr: string }) => {
  const { data } = useGetMrTask(mr)
  const [buildid, setBuildId] = useAtom(buildId)
  const { logsMap, setEventSource } = useTaskSSE()
  const [loading] = useAtom(loadingAtom)
  const [statusMap, _setStatusMap] = useAtom(statusMapAtom)
  const { data: status } = useGetMrTaskStatus(mr)

  useEffect(() => {
    status &&
      status.map((i) => {
        statusMap.set(i.build_id, i)
      })
  }, [status, statusMap])

  // 页面加载时建立连接
  useEffect(() => {
    if (data) {
      setBuildId(data[0].build_id)
      data.map((i) => setEventSource(i.build_id))
    }
    setBuildId(mocks[0].build_id)
    mocks.map((i) => setEventSource(i.build_id))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return (
    <>
      <div className='bg-[#f6f8fa]' style={{ height: `calc(100vh - 104px)` }}>
        <div className='flex h-[60px] items-center border-b bg-white px-4'>
          <span>
            <h2 className='text-bold fz-[14px] text-[#59636e]'>[] tasks status interface</h2>
          </span>
        </div>
        <div className='flex justify-between' style={{ height: `calc(100vh - 164px)` }}>
          {/* left side */}
          <div className='h-full w-[40%] border-r'>
            {/* {data &&  <Task list={data} />} */}
            <Task list={data as TaskResult[]} />
          </div>
          {/* right side */}
          <div className='flex-1'>
            {logsMap[buildid] ? (
              <LazyLog
                extraLines={1}
                text={(logsMap[buildid] ?? []).join('\n')}
                stream
                enableSearch
                caseInsensitive
                follow
              />
            ) : (
              loading && (
                <div className='flex h-full flex-1 items-center justify-center'>
                  <LoadingSpinner />
                </div>
              )
            )}
          </div>
        </div>
      </div>
    </>
  )
}

export default memo(Checks)
