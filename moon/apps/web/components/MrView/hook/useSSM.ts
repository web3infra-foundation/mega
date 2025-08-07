export const useSSM = () => {
  const createEventSource = (baseUrl: string): Promise<EventSource> => {
    return new Promise<EventSource>((res, rej) => {
      const es = new EventSource(baseUrl)

      es.onopen = () => {
        res(es)
      }
      es.onerror = () => {
        rej('eventsource建立失败')
      }
    })
  }

  return {
    createEventSource
  }
}
