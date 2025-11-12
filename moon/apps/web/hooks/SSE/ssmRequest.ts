import { GetTaskBuildListByIdData } from '@gitmono/types/generated'

import { orionApiClient } from '@/utils/queryClient'

const getBaseUrl = () => (orionApiClient as any).baseUrl || ''

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

export const fetchAllbuildList = async (id: string): Promise<GetTaskBuildListByIdData> => {
  const res = await fetch(`${getBaseUrl()}/task-build-list/${id}`, {
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
