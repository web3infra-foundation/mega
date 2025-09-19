'use client'

import { memo, useState } from 'react'
import { TextInput } from '@primer/react'

import { Button } from '@gitmono/ui/Button'
import { LoadingSpinner } from '@gitmono/ui/Spinner'
import { UIText } from '@gitmono/ui/Text'

import { usePostIssueTitle } from '@/hooks/issues/usePostIssueTitle'
import { usePostMrTitle } from '@/hooks/MR/usePostMrTitle'
import { apiErrorToast } from '@/utils/apiErrorToast'

const TitleInput = ({
  title,
  id,
  whoami,
  callback
}: {
  title: string
  id: string
  whoami: 'mr' | 'issue'
  callback?: () => void | Promise<any>
}) => {
  const [editTitle, setEditTitle] = useState(title)
  const [isEdit, setIsEdit] = useState(false)
  const [loading, setLoading] = useState(false)
  const { mutate: modifyMRTitle } = usePostMrTitle()
  const { mutate: modifyIssueTitle } = usePostIssueTitle()

  const handleSave = () => {
    if (editTitle === title || editTitle === '') {
      apiErrorToast(new Error('Nothing Changed or is Empty'))
      return
    }
    setLoading(true)
    switch (whoami) {
      case 'mr':
        modifyMRTitle(
          { link: id, data: { content: editTitle as string } },
          {
            onError: (err) => apiErrorToast(err),
            onSuccess: async () => {
              await callback?.()
              setIsEdit(false)
              setLoading(false)
            }
          }
        )
        break
      case 'issue':
        modifyIssueTitle(
          { link: id, data: { content: editTitle as string } },
          {
            onError: (err) => apiErrorToast(err),
            onSuccess: async () => {
              await callback?.()
              setIsEdit(false)
              setLoading(false)
            }
          }
        )
        break

      default:
        break
    }
  }

  return (
    <>
      <div className='mb-2 w-[70%]'>
        {!isEdit && (
          <>
            <div className='flex w-full items-center justify-between'>
              <UIText size='text-2xl' weight='font-bold' className='-tracking-[1px] lg:flex'>
                {`${title || ''}`}
                <span>&nbsp;</span>
                <span className='font-light !text-[#59636e]'>${id}</span>
              </UIText>
              <Button
                onClick={() => {
                  setEditTitle(title)
                  setIsEdit(true)
                }}
              >
                Edit
              </Button>
            </div>
          </>
        )}
        {isEdit && (
          <>
            <div className='flex w-full items-center justify-between gap-2'>
              <TextInput
                value={editTitle || ''}
                onChange={(e) => {
                  setEditTitle(e.target.value)
                }}
                className='new-issue-input no-border-input w-[80%]'
                trailingVisual={() => (loading ? <LoadingSpinner /> : '')}
              />
              <div className='flex gap-4'>
                <Button onClick={handleSave}>Save</Button>
                <Button onClick={() => setIsEdit(false)}>Cancel</Button>
              </div>
            </div>
          </>
        )}
      </div>
    </>
  )
}

export default memo(TitleInput)
