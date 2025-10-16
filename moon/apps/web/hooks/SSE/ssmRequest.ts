import {
  GetTaskBuildListByIdData,
  GetTaskHistoryOutputByIdData,
  GetTaskHistoryOutputByIdParams,
  GetTasksByClData
} from '@gitmono/types/generated'

import { SSEPATH } from '@/components/ClView/hook/useSSM'

export const fetchTask = async (cl: number): Promise<GetTasksByClData> => {
  const res = await fetch(`${SSEPATH}/tasks/${cl}`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json'
    }
  })

  if (!res.ok) {
    throw new Error(`HTTP error ${res.status}`)
  }
  return res.json()
}

export const taskStatus = async (taskId: string) => {
  const res = await fetch(`/sse/task-status/${taskId}`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json'
    }
  })

  if (!res.ok) {
    throw new Error(`HTTP error ${res.status}`)
  }
  return res.json()
}

export const ClTaskStatus = async (cl: string) => {
  const res = await fetch(`/sse/tasks/${cl}`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json'
    }
  })

  if (!res.ok) {
    throw new Error(`HTTP error ${res.status}`)
  }
  return res.json()
}

export const HttpTaskRes = async (payload: GetTaskHistoryOutputByIdParams): Promise<GetTaskHistoryOutputByIdData> => {
  const res = await fetch(
    `${SSEPATH}task-history-output/${payload.id}?type=${payload.type}?offset=${payload.offset}&limit=${payload.limit}`,
    {
      // const res = await fetch(`/sse/task-output-segment/${taskId}?offset=${offset}&len=${len}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json'
      }
    }
  )

  if (!res.ok) {
    throw new Error(`HTTP error ${res.status}`)
  }
  return res.json()
}

export const fetchAllbuildList = async (id: string): Promise<GetTaskBuildListByIdData> => {
  const res = await fetch(`${SSEPATH}task-build-list/${id}`, {
    // const res = await fetch(`/sse/task-output-segment/${taskId}?offset=${offset}&len=${len}`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json'
    }
  })

  if (!res.ok) {
    throw new Error(`HTTP error ${res.status}`)
  }
  return res.json()
}
