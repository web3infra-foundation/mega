import { ItemInput } from '@primer/react/lib/deprecated/ActionList'

export function mergeAndDeduplicate(
  prev: ItemInput[],
  selected: ItemInput[],
  prevId: string,
  selectId: string
): ItemInput[] {
  const updatedPrev = prev.map((item) => ({
    ...item,
    groupId: prevId
  }))

  const updatedSelected = selected.map((item) => ({
    ...item,
    groupId: selectId
  }))

  const mergedArray = [...updatedPrev, ...updatedSelected]

  return Array.from(new Map(mergedArray.map((item) => [item.text, item])).values())
}
