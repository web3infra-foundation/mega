import { useEffect } from 'react'
import { LazyLog } from '@melloware/react-logviewer'
import { useAtom } from 'jotai'

import { LoadingSpinner } from '@gitmono/ui/Spinner'

import { buildIdAtom } from '@/components/Issues/utils/store'
import { HttpTaskRes } from '@/hooks/SSE/ssmRequest'
// import { TaskResult } from '@/hooks/SSE/useGetMrTask'
import { useGetMrTaskStatus } from '@/hooks/SSE/useGetMrTaskStatus'

import { useTaskSSE } from '../../hook/useSSM'
import { statusMapAtom } from './cpns/store'
import { Task } from './cpns/Task'

const Checks = ({ mr }: { mr: string }) => {
  // const { data } = useGetMrTask(mr)
  const [buildid, setBuildId] = useAtom(buildIdAtom)
  const { logsMap, setEventSource, eventSourcesRef, setLogsMap } = useTaskSSE()
  const [statusMap, _setStatusMap] = useAtom(statusMapAtom)
  // 获取所有构建任务
  const { data: status } = useGetMrTaskStatus(mr)

  useEffect(() => {
    // 构建日志id与日志映射，同时获取已存在日志
    if (!status) return
    // setBuildId((prev) => prev ?? status[0].build_id)
    const fetchLogs = async () => {
      const logsResult = await Promise.allSettled(
        status.map(async (i) => {
          statusMap.set(i.build_id, i)
          const res = await HttpTaskRes(i.build_id, 0, 4096)

          return res
        })
      )
      const newLogsMap = logsResult.reduce(
        (acc, i) => {
          if (i.status === 'fulfilled' && i.value) {
            acc[i.value.task_id] = i.value.data === '' ? 'empty logs, please check it later' : i.value.data
          }
          return acc
        },
        { ...logsMap }
      )

      setLogsMap(newLogsMap)
    }

    fetchLogs()
    setBuildId(status[0].build_id)
    return () => {
      statusMap.clear()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status])

  // 页面加载时建立连接
  useEffect(() => {
    if (status?.length) {
      status.map((i) => setEventSource(i.build_id))
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status])

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
            {status && <Task list={status} />}
            {/* <Task list={status as TaskResult[]} /> */}
          </div>
          {/* right side */}
          <div className='flex-1'>
            {
              logsMap[buildid] && eventSourcesRef.current[buildid] ? (
                <LazyLog extraLines={1} text={logsMap[buildid]} stream enableSearch caseInsensitive follow />
              ) : eventSourcesRef.current[buildid] ? (
                <div></div>
              ) : (
                // <LazyLog extraLines={1} text={logsMap[buildid]} stream enableSearch caseInsensitive follow />
                <div className='flex h-full flex-1 items-center justify-center'>
                  <LoadingSpinner />
                </div>
              )

              // loading && (
              //   <div className='flex h-full flex-1 items-center justify-center'>
              //     <LoadingSpinner />
              //   </div>
              // )
            }
          </div>
        </div>
      </div>
    </>
  )
}

// export default memo(Checks)
export default Checks
