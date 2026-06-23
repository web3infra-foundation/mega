import { useCallback, useEffect, useRef } from 'react'
import { atom, useAtom } from 'jotai'
import { atomFamily } from 'jotai/utils'
import { useRouter } from 'next/router'

import {
  buildIdAtomFamily,
  findTaskIdByBuildId,
  getAllBuildIds,
  getLatestBuildId,
  getLatestBuildIdFromTasks,
  TaskInfoDTO
} from '../cpns/store'
import { readBuildFromUrl, syncChecksUrl } from './syncChecksUrl'

const selectedTaskIdAtomFamily = atomFamily((_cl: string) => atom<string | null>(null))

export function useBuildSelection(cl: string, tasks: TaskInfoDTO[] | undefined) {
  const router = useRouter()
  const [buildId, setBuildId] = useAtom(buildIdAtomFamily(cl))
  const [selectedTaskId, setSelectedTaskId] = useAtom(selectedTaskIdAtomFamily(cl))
  const taskBuildMemoryRef = useRef<Map<string, string>>(new Map())
  const prevClRef = useRef(cl)
  const syncedBuildRef = useRef<string | null>(null)

  useEffect(() => {
    if (prevClRef.current === cl) return

    prevClRef.current = cl
    syncedBuildRef.current = null
    taskBuildMemoryRef.current.clear()
    setBuildId('')
    setSelectedTaskId(null)
  }, [cl, setBuildId, setSelectedTaskId])

  useEffect(() => {
    if (!router.isReady || !tasks || tasks.length === 0) return

    const allBuildIds = getAllBuildIds(tasks)
    const queryBuild = readBuildFromUrl(router.query.build)

    let targetBuildId: string | undefined

    if (queryBuild && allBuildIds.has(queryBuild)) {
      targetBuildId = queryBuild
    } else if (buildId && allBuildIds.has(buildId)) {
      targetBuildId = buildId
    } else {
      targetBuildId = getLatestBuildIdFromTasks(tasks)
    }

    if (targetBuildId && targetBuildId !== buildId) {
      setBuildId(targetBuildId)
    }

    const taskId = targetBuildId ? findTaskIdByBuildId(tasks, targetBuildId) : undefined
    const validTasks = tasks.filter((t) => t.build_list && t.build_list.length > 0)

    if (taskId && taskId !== selectedTaskId) {
      setSelectedTaskId(taskId)
    } else if (!selectedTaskId && validTasks.length > 0) {
      setSelectedTaskId(validTasks[0].task_id)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tasks, router.isReady, router.query.build, cl])

  useEffect(() => {
    if (!buildId) return

    if (syncedBuildRef.current === buildId) return

    const urlBuild = typeof window !== 'undefined' ? new URLSearchParams(window.location.search).get('build') : null
    const urlTab = typeof window !== 'undefined' ? new URLSearchParams(window.location.search).get('tab') : null

    if (urlTab === 'check' && urlBuild === buildId) {
      syncedBuildRef.current = buildId
      return
    }

    syncedBuildRef.current = buildId
    syncChecksUrl(buildId)
  }, [buildId])

  const selectBuild = useCallback(
    (nextBuildId: string, taskId?: string) => {
      if (taskId) {
        taskBuildMemoryRef.current.set(taskId, nextBuildId)
      }

      setBuildId(nextBuildId)
    },
    [setBuildId]
  )

  const selectTask = useCallback(
    (taskId: string) => {
      setSelectedTaskId(taskId)

      const task = tasks?.find((t) => t.task_id === taskId)

      if (!task) return

      const remembered = taskBuildMemoryRef.current.get(taskId)
      const rememberedValid = remembered && task.build_list?.some((b) => b.id === remembered)

      const nextBuildId = rememberedValid ? remembered : getLatestBuildId(task)

      if (nextBuildId) {
        taskBuildMemoryRef.current.set(taskId, nextBuildId)
        setBuildId(nextBuildId)
      }
    },
    [tasks, setBuildId, setSelectedTaskId]
  )

  return {
    buildId,
    selectBuild,
    selectedTaskId,
    selectTask
  }
}
