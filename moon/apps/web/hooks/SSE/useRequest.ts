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
