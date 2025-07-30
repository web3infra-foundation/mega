import { useMemo, useRef, useState } from 'react'
import { ItemInput } from '@primer/react/lib/SelectPanel/types'

import { MemberAvatar } from '@/components/MemberAvatar'
import { useSyncedMembers } from '@/hooks/useSyncedMembers'

import { extractTextArray } from './extractText'
import { useGetLabelList } from '@/hooks/useGetLabelList'

export const useAvatars = () => {
  const { members } = useSyncedMembers()

  return useMemo(
    () =>
      members?.map((i) => ({
        groupId: 'end',
        text: i.user.username,
        leadingVisual: () => <MemberAvatar size='sm' member={i} />
      })) || [],
    [members]
  )
}

export const splitFun = (el: React.ReactNode): string[] => {
  return extractTextArray(el)
    .flatMap((name) => name.split(',').map((n) => n.trim()))
    .filter((n) => n.length > 0)
}

export const useMemberMap = () => {
  const { members } = useSyncedMembers()

  return useMemo(() => {
    const map = new Map()

    members?.forEach((i) => {
      map.set(i.user.username, i)
    })
    return map
  }, [members])
}

export const useLabels = () => {
  const { labels } = useGetLabelList()

  return useMemo(
    () =>
      labels.map((i) => ({
        text: i.description,
        leadingVisual: () => (
          <div
            className='h-[14px] w-[14px] rounded-full border'
            //eslint-disable-next-line react/forbid-dom-props
            style={{ backgroundColor: i.color, borderColor: i.color }}
          />
        )
      })),
    [labels]
  )
}

export const useLabelMap = () => {
  const { labels } = useGetLabelList()

  return useMemo(() => {
    const map = new Map()

    labels.map((i) => {
      map.set(i.description, i)
    })
    return map
  }, [labels])
}

// assignees逻辑

export const useAssigneesSelector = ({
  assignees,
  assignRequest,
  avatars
}: {
  assignees: string[]
  assignRequest: (selected: string[]) => void
  avatars: ReturnType<typeof useAvatars>
}) => {
  const selectRef = useRef<string[]>([])
  let selects: string[] = assignees as string[]
  const shouldFetch = useRef(false)
  const [open, setOpen] = useState(false)

  const handleAssignees = (selected: ItemInput[]) => {
    selects = [...selected.map((i) => i.text).filter((t): t is string => typeof t === 'string')]
  }

  const handleOpenChange = (open: boolean) => {
    if (selectRef.current.length !== selects.length) {
      shouldFetch.current = true
    } else {
      const set = new Set(selects)

      for (let i = 0; i < selectRef.current.length; i++) {
        if (!set.has(selectRef.current[i])) {
          shouldFetch.current = true
          break
        }
      }
    }

    setOpen(open)
    if (!open && shouldFetch.current) {
      selectRef.current = selects
      assignRequest(selectRef.current)
      setTimeout(() => (shouldFetch.current = false), 0)
    }
  }

  const fetchSelected = useMemo(() => {
    const set = new Set(selectRef.current.length ? selectRef.current : assignees)

    return avatars.filter((user) => set.has(user.text as string))
  }, [selectRef, avatars, assignees])

  return {
    open,
    handleAssignees,
    handleOpenChange,
    fetchSelected
  }
}

export const useChange = ({ title = 'Close issue' }: { title?: string }) => {
  const [closeHint, setCloseHint] = useState(title)

  const needComment = useRef(false)
  const handleChange = (html: string) => {
    if (html && html === '<p></p>') {
      setCloseHint(title)
    } else {
      setCloseHint('Close with comment')
    }
  }

  const handleCloseChange = (html: string) => {
    if (html && html === '<p></p>') {
      needComment.current = false
    } else {
      needComment.current = true
    }
  }

  return {
    closeHint,
    needComment,
    handleChange,
    handleCloseChange
  }
}
