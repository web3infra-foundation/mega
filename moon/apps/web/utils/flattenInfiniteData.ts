import { InfiniteData } from '@tanstack/react-query'

interface Identifiable {
  id: string
}
interface DataPage<T> {
  data: T[]
}
interface Page<T> {
  [key: string]: T[]
}

function _flattenInfiniteData<T extends Identifiable, K extends keyof Page<T>>(
  dataKey: K,
  data?: InfiniteData<Page<T>>
) {
  const ids = new Set()

  return data?.pages
    .map((page) => page[dataKey])
    .flat(2)
    .filter((obj) => {
      if (ids.has(obj.id)) {
        return false
      } else {
        ids.add(obj.id)
        return true
      }
    })
}

export function flattenInfiniteData<T extends { id: string }>(data?: InfiniteData<DataPage<T>>) {
  // return _flattenInfiniteData('data', data)
  return _flattenInfiniteData('data', data as unknown as InfiniteData<Page<T>>)
}
