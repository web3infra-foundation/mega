import { forwardRef, useCallback, useImperativeHandle, useMemo, useRef, useState } from 'react'
import { useAtom } from 'jotai'
import toast from 'react-hot-toast'
import { useRouter } from 'next/router'

import { getNoteExtensions } from '@gitmono/editor'
import { ConversationItem } from '@gitmono/types/generated'
import { Button, ConditionalWrap, FaceSmilePlusIcon, PicturePlusIcon, UIText } from '@gitmono/ui'

import { EMPTY_HTML } from '@/atoms/markdown'
import { RichTextRenderer } from '@/components/RichTextRenderer'
import { usePostComment } from '@/hooks/issues/usePostComment'
import { useGetOrganizationMember } from '@/hooks/useGetOrganizationMember'
import { useGetCurrentUser } from '@/hooks/useGetCurrentUser'
import { usePostMrReviewResolve } from '@/hooks/usePostMrReviewResolve'
import { useQueryClient } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { useUploadHelpers } from '@/hooks/useUploadHelpers'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { trimHtml } from '@/utils/trimHtml'

import { MemberHovercard } from '../InlinePost/MemberHovercard'
import { useChange } from '../Issues/utils/sideEffect'
import { editIdAtom, FALSE_EDIT_VAL } from '../Issues/utils/store'
import { MemberAvatar } from '../MemberAvatar'
import { useHandleBottomScrollOffset } from '../NoteEditor/useHandleBottomScrollOffset'
import { ComposerReactionPicker } from '../Reactions/ComposerReactionPicker'
import { ReactionPicker } from '../Reactions/ReactionPicker'
import { ReactionShow } from '../Reactions/ReactionShow'
import { SimpleNoteContent, SimpleNoteContentRef } from '../SimpleNoteEditor/SimpleNoteContent'
import { CommentDropdownMenu } from './CommentDropdownMenu'
import HandleTime from './components/HandleTime'
import { UserLinkByName } from './components/UserLinkByName'
import { useHandleExpression } from './hook/useHandleExpression'

interface ReviewCommentProps {
  assignees: string[]
  conv: ConversationItem
  id: string
  whoamI: string
  editorRef: React.RefObject<SimpleNoteContentRef>
}

const ReviewComment = ({assignees, conv, id, whoamI, editorRef }: ReviewCommentProps) => {
  const { data: member } = useGetOrganizationMember({ username: conv.username })
  const { data: currentUser } = useGetCurrentUser()
  const { mutate: resolveReview } = usePostMrReviewResolve()
  const queryClient = useQueryClient()
  const router = useRouter()
  const { link } = router.query
  const mrLink = typeof link === 'string' ? link : ''

  const extensions = useMemo(() => getNoteExtensions({ linkUnfurl: {} }), [])
  const handleReactionSelect = useHandleExpression({ conv, id, type: whoamI })
  const [editId, setEditId] = useAtom(editIdAtom)
  const editInputRef = useRef<{ handleUpdate: () => void }>()
  const [isResolved, setIsResolved] = useState(false)

  const canResolve = useMemo(() => {
    if (!currentUser?.username) {
      return false
    }
    return assignees.includes(currentUser.username)
  }, [currentUser?.username, assignees])

  const handleResolve = useCallback(async () => {
    try {
      await new Promise<void>((resolve, reject) => {
        resolveReview({
          link: mrLink,
          data: {
            conversation_id: conv.id,
            resolved: true
          }
        }, {
          onSuccess: () => {
            setIsResolved(true)
            toast.success('Review resolved successfully!')

            queryClient.invalidateQueries({
              queryKey: legacyApiClient.v1.getApiMrDetail().requestKey(id)
            })

            resolve()
          },
          onError: (error: any) => {
            apiErrorToast(new Error(error.message))
            reject(error)
          }
        })
      })
    } catch (error) {
      // Error already handled in onError callback
    }
  }, [resolveReview, mrLink, conv.id, queryClient, id])

  return (
    <div className={`overflow-hidden rounded-lg border-2 bg-white ${
      isResolved ? 'border-green-200 bg-green-50' : 'border-blue-200 bg-blue-50'
    }`}>
      <div className="bg-blue-100 px-3 py-1 text-xs font-medium text-blue-800">
        📝 Review Comment
        {isResolved && (
          <span className="ml-2 inline-flex items-center rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-800">
            ✓ Resolved
          </span>
        )}
      </div>

      <div className='flex items-center justify-between border-b border-blue-200 px-4 py-2'>
        <div className='flex items-center space-x-3'>
          <div className='cursor-pointer'>
            <ConditionalWrap
              condition={true}
              wrap={(c) => (
                <MemberHovercard username={conv?.username}>
                  <UserLinkByName username={conv?.username} className='relative'>
                    {c}
                  </UserLinkByName>
                </MemberHovercard>
              )}
            >
              {member ? <MemberAvatar member={member} size='sm' /> : 'Avatar not found'}
            </ConditionalWrap>
          </div>
          <div className='cursor-pointer'>
            <ConditionalWrap
              condition={true}
              wrap={(children) => (
                <MemberHovercard username={conv?.username}>
                  <UserLinkByName username={conv?.username}>{children}</UserLinkByName>
                </MemberHovercard>
              )}
            >
              <UIText element='span' primary weight='font-medium' className='break-anywhere line-clamp-1'>
                {conv?.username || 'username not found'}
              </UIText>
            </ConditionalWrap>
          </div>
          <div className='text-sm text-gray-500 hover:text-gray-700'>
            <HandleTime created_at={conv.created_at} />
          </div>
        </div>
        <div className='flex items-center space-x-2'>
          {canResolve && !isResolved && (
            <Button
              size="sm"
              variant="base"
              onClick={handleResolve}
              className="text-green-600 border-green-300 hover:bg-green-50"
            >
              Resolve Conversation
            </Button>
          )}
          <ReactionPicker
            custom
            align='end'
            trigger={
              <Button
                variant='plain'
                iconOnly={<FaceSmilePlusIcon />}
                accessibilityLabel='Add reaction'
                tooltip='Add reaction'
              />
            }
            onReactionSelect={handleReactionSelect}
          />
          <CommentDropdownMenu id={id} Conversation={conv} CommentType={whoamI} editorRef={editorRef} />
        </div>
      </div>

      <div className='prose copyable-text p-3'>
        {conv.comment && editId !== conv.id && <RichTextRenderer content={conv.comment} extensions={extensions} />}
        {editId === conv.id && (
          <>
            <EditInput ref={editInputRef} comment={conv} />
            <div className='flex justify-end gap-4'>
              <Button onClick={() => setEditId(FALSE_EDIT_VAL)}>
                Cancel
              </Button>
              <Button
                onClick={() => {
                  editInputRef.current && editInputRef.current.handleUpdate()
                }}
                className='bg-[#1f883d] text-white'
              >
                Update Comment
              </Button>
            </div>
          </>
        )}
        <ReactionShow comment={conv} id={id} type={whoamI} />
      </div>
    </div>
  )
}

const EditInput = forwardRef(({ comment }: { comment: ConversationItem }, ref) => {
  const editorRef = useRef<SimpleNoteContentRef>(null)
  const onKeyDownScrollHandler = useHandleBottomScrollOffset({
    editor: editorRef.current?.editor
  })
  const { dropzone } = useUploadHelpers({
    upload: editorRef.current?.uploadAndAppendAttachments
  })
  const { handleChange } = useChange({})
  const [isReactionPickerOpen, setIsReactionPickerOpen] = useState(false)
  const { mutateAsync: updateComment } = usePostComment()
  const [isUpdating, setIsUpdating] = useState(false)

  const handleUpdate = async () => {
    if (isUpdating) return

    const currentContentHTML = editorRef.current?.editor?.getHTML() ?? '<p></p>'

    if (trimHtml(currentContentHTML) === '') {
      toast.error('comment can not be empty!')
      return
    }

    setIsUpdating(true)
    try {
      await updateComment({ commentId: comment.id, data: { content: currentContentHTML } })
      toast.success('update successfully!')
    } catch (err: any) {
      apiErrorToast(new Error(err.message))
    } finally {
      setIsUpdating(false)
    }
  }

  useImperativeHandle(ref, () => ({
    handleUpdate
  }))

  return (
    <>
      <div className='prose mt-4 flex w-full flex-col'>
        <input {...dropzone.getInputProps()} />
        <div className='relative rounded-lg border p-6 pb-12'>
          <SimpleNoteContent
            commentId='temp'
            ref={editorRef}
            editable='all'
            content={comment.comment || EMPTY_HTML}
            autofocus={true}
            onKeyDown={onKeyDownScrollHandler}
            onChange={(html) => handleChange(html)}
          />
          <Button
            variant='plain'
            iconOnly={<PicturePlusIcon />}
            accessibilityLabel='Add files'
            onClick={dropzone.open}
            tooltip='Add files'
          />
          <ComposerReactionPicker
            editorRef={editorRef}
            open={isReactionPickerOpen}
            onOpenChange={setIsReactionPickerOpen}
          />
        </div>
      </div>
    </>
  )
})

EditInput.displayName = 'EditInput'

export default ReviewComment