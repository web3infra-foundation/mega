import { memo, useEffect, useRef } from 'react'
import { LazyLog } from '@melloware/react-logviewer'
import { useAtom } from 'jotai'

import { buildIdAtom } from '@/components/Issues/utils/store'
import { fetchAllbuildList, HttpTaskRes } from '@/hooks/SSE/ssmRequest'
import { useGetMrTask } from '@/hooks/SSE/useGetMrTask'

// import { TaskResult } from '@/hooks/SSE/useGetMrTask'

import { useTaskSSE } from '../../hook/useSSM'
import { statusMapAtom } from './cpns/store'
import { Task } from './cpns/Task'

const Checks = ({ mr }: { mr: number }) => {
  // const { data } = useGetMrTask(mr)
  const [buildid, _setBuildId] = useAtom(buildIdAtom)
  const { logsMap, setEventSource, eventSourcesRef, setLogsMap } = useTaskSSE()
  const [statusMap, _setStatusMap] = useAtom(statusMapAtom)
  const tasksId = useRef<string[]>([])
  const allBUildIds = useRef<string[]>([])
  // 获取所有构建任务
  // const { data: status } = useGetMrTaskStatus(mr)
  const { data: tasks } = useGetMrTask(mr)

  tasks && (tasksId.current = tasks.map((i) => i.task_id))

  const establish = async () => {
    const statusList = await Promise.allSettled(tasksId.current.map(async (i) => fetchAllbuildList(i)))

    allBUildIds.current = statusList
      .map((i) => {
        if (i.status === 'fulfilled' && i.value) {
          return i.value
        }
        return []
      })
      .flat()

    allBUildIds.current.map((id) => setEventSource(id))
  }

  // 进入页面建立连接

  // const { data: tasks } = useGetMrTask(2816411452522757)

  // 进入页面建立所有连接
  // 进入页面时先获取所有的构建任务
  // useEffect(()=>{
  //   const {} =
  // },[])

  useEffect(() => {
    if (!allBUildIds.current.length) return
    establish()
    // 构建日志id与日志映射，同时获取已存在日志

    // setBuildId((prev) => prev ?? status[0].build_id)
    const fetchLogs = async () => {
      const logsResult = await Promise.allSettled(
        allBUildIds.current.map(async (id) => {
          // statusMap.set(i.build_id, i)
          const res = await HttpTaskRes({ id, type: 'full' })

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
    // setBuildId(status[0].build_id)
    return () => {
      statusMap.clear()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // 页面加载时建立连接
  // useEffect(() => {
  //   if (status?.length) {
  //     status.map((i) => setEventSource(i.build_id))
  //   }
  //   // eslint-disable-next-line react-hooks/exhaustive-deps
  // }, [status])

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
            {/* {mockTasksList && mockTasksList.map((t) => <Task key={t.task_id} list={t} />)} */}
            {tasks && tasks.map((t) => <Task key={t.task_id} list={t} />)}
            {/* <Task list={status as TaskResult[]} /> */}
          </div>
          {/* right side */}
          <div className='flex-1'>
            {
              logsMap[buildid] && eventSourcesRef.current[buildid] && (
                <LazyLog extraLines={1} text={logsMap[buildid]} stream enableSearch caseInsensitive follow />
              )
              // : eventSourcesRef.current[buildid] ? (
              //   <div></div>
              // ) : (
              //   // <LazyLog extraLines={1} text={logsMap[buildid]} stream enableSearch caseInsensitive follow />
              //   <div className='flex h-full flex-1 items-center justify-center'>
              //     <LoadingSpinner />
              //   </div>
              // )

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

export default memo(Checks)
