import { HTTPLogRes } from './useGetHTTPLog'

export const fetchTask = async (mr: string) => {
  const res = await fetch(`/sse/mr-task/${mr}`, {
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

export const MrTaskStatus = async (mr: string) => {
  const res = await fetch(`/sse/tasks/${mr}`, {
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

export const HttpTaskRes = async (taskId: string, offset: number, len: number): Promise<HTTPLogRes> => {
  const res = await fetch(`/sse/task-output-segment/${taskId}?offset=${offset}&len=${len}`, {
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
