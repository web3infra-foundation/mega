import { memo, useEffect } from 'react'
import { LazyLog } from '@melloware/react-logviewer'
import { useAtom } from 'jotai'

import { buildIdAtom } from '@/components/Issues/utils/store'
import { useGetClTask } from '@/hooks/SSE/useGetClTask'
import { fetchHTTPLog } from '@/hooks/SSE/useGetHTTPLog'

import { useTaskSSE } from '../../hook/useSSM'
import { statusMapAtom } from './cpns/store'
import { Task } from './cpns/Task'

const Checks = ({ cl }: { cl: number }) => {
  const [buildid, setBuildId] = useAtom(buildIdAtom)
  const { logsMap, setEventSource, eventSourcesRef, setLogsMap } = useTaskSSE()
  const [statusMap, _setStatusMap] = useAtom(statusMapAtom)
  const { data: tasks } = useGetClTask(cl)

  useEffect(() => {
    if (!tasks || tasks.length === 0) return

    const allBuildIds = tasks.flatMap((task) => task.build_list.map((build) => build.id))

    if (allBuildIds.length === 0) return

    allBuildIds.forEach((id) => setEventSource(id))

    const fetchLogs = async () => {
      const logsResult = await Promise.allSettled(
        allBuildIds.map(async (id) => {
          const res = await fetchHTTPLog({ id, type: 'full' })

          return { id, res }
        })
      )

      const newLogsMap = logsResult.reduce(
        (acc, item) => {
          if (item.status === 'fulfilled' && item.value) {
            const { id, res } = item.value
            const logText = Array.isArray(res.data)
              ? res.data.join('\n')
              : (res.data || 'empty logs, please check it later')

            acc[id] = logText
          }
          return acc
        },
        {} as Record<string, string>
      )

      setLogsMap(newLogsMap)
    }

    fetchLogs()

    if (!buildid && allBuildIds.length > 0) {
      setBuildId(allBuildIds[0])
    }

    return () => {
      statusMap.clear()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tasks])

  return (
    <>
      <div className='bg-[#f6f8fa]' style={{ height: `calc(100vh - 104px)` }}>
        <div className='flex h-[60px] items-center border-b bg-white px-4'>
          <span>
            <h2 className='text-bold fz-[14px] text-[#59636e]'>[] tasks status interface</h2>
          </span>
        </div>
        <div className='flex justify-between' style={{ height: `calc(100vh - 164px)` }}>
          <div className='h-full w-[40%] border-r'>
            {tasks && tasks.map((t) => <Task key={t.task_id} list={t} />)}
          </div>
          <div className='flex-1'>
            {buildid ? (
              logsMap[buildid] && eventSourcesRef.current[buildid] ? (
                <LazyLog extraLines={1} text={logsMap[buildid]} stream enableSearch caseInsensitive follow />
              ) : (
                <div className='flex h-full items-center justify-center text-gray-500'>
                  <span>Loading logs...</span>
                </div>
              )
            ) : (
              <div className='flex h-full items-center justify-center text-gray-500'>
                <span>Select a build to view logs</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </>
  )
}

export default memo(Checks)
