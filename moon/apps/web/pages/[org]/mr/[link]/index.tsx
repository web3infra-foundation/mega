'use client'

import React, { useEffect, useRef, useState } from 'react'
import { BaseStyles, TextInput, ThemeProvider } from '@primer/react'
import { useAtom } from 'jotai'
import dynamic from 'next/dynamic'
import { useRouter } from 'next/router'
import { toast } from 'react-hot-toast'

import { ConversationItem } from '@gitmono/types/generated'
import { Button, LoadingSpinner, UIText } from '@gitmono/ui'
import { PicturePlusIcon } from '@gitmono/ui/Icons'
import { cn } from '@gitmono/ui/utils'

import { EMPTY_HTML } from '@/atoms/markdown'
import FileDiff from '@/components/DiffView/FileDiff'
import { BadgeItem } from '@/components/Issues/IssueNewPage'
import {
  splitFun,
  useAssigneesSelector,
  useAvatars,
  useChange,
  useLabelMap,
  useLabels,
  useLabelsSelector,
  useMemberMap
} from '@/components/Issues/utils/sideEffect'
import { editIdAtom, FALSE_EDIT_VAL, mridAtom, refreshAtom } from '@/components/Issues/utils/store'
import { AppLayout } from '@/components/Layout/AppLayout'
import { TabLayout } from '@/components/Layout/TabLayout'
import { MemberAvatar } from '@/components/MemberAvatar'
import { tabAtom } from '@/components/MrView/components/Checks/cpns/store'
import TimelineItems from '@/components/MrView/TimelineItems'
import { useHandleBottomScrollOffset } from '@/components/NoteEditor/useHandleBottomScrollOffset'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { ComposerReactionPicker } from '@/components/Reactions/ComposerReactionPicker'
import { SimpleNoteContent, SimpleNoteContentRef } from '@/components/SimpleNoteEditor/SimpleNoteContent'
import { useScope } from '@/contexts/scope'
import { usePostMRAssignees } from '@/hooks/issues/usePostMRAssignees'
import { usePostMrTitle } from '@/hooks/MR/usePostMrTitle'
import { useGetMrDetail } from '@/hooks/useGetMrDetail'
import { useMrFilesChanged } from '@/hooks/useMrFilesChanged'
import { usePostMrClose } from '@/hooks/usePostMrClose'
import { usePostMrComment } from '@/hooks/usePostMrComment'
import { usePostMRLabels } from '@/hooks/usePostMRLabels'
// import { usePostMrMerge } from '@/hooks/usePostMrMerge'
import { usePostMrReopen } from '@/hooks/usePostMrReopen'
import { useUploadHelpers } from '@/hooks/useUploadHelpers'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { trimHtml } from '@/utils/trimHtml'
import { PageWithLayout } from '@/utils/types'
import { MergeBox } from "@/components/MrBox/MergeBox";

export interface MRDetail {
  status: string
  conversations: ConversationItem[]
  title: string
}

const MRDetailPage: PageWithLayout<any> = () => {
  const router = useRouter()
  const { link: tempId } = router.query
  const [item_id] = useAtom(mridAtom)
  const { scope } = useScope()
  const [login, _setLogin] = useState(true)
  const [isReactionPickerOpen, setIsReactionPickerOpen] = useState(false)
  const id = typeof tempId === 'string' ? tempId : ''
  const { data: MrDetailData, isLoading: detailIsLoading, refetch } = useGetMrDetail(id)
  const mrDetail = MrDetailData?.data as MRDetail | undefined
  const { closeHint, needComment, handleChange } = useChange({ title: 'Close Merge Request' })
  const { mutate: mrAssignees } = usePostMRAssignees()
  const { mutate: mrLabels } = usePostMRLabels()
  const [isEdit, setIsEdit] = useState(false)
  const [editTitle, setEditTitle] = useState(mrDetail?.title)
  const [loading, setLoading] = useState(false)
  const Checks = dynamic(() => import('@/components/MrView/components/Checks'))
  const [page, _setPage] = useState(1)

  if (mrDetail) {
    mrDetail.status = mrDetail.status.toLowerCase()
  }

  const { data: MrFilesChangedData, isLoading: fileChgIsLoading } = useMrFilesChanged(id, {
    additional: 'string', 
    pagination: {
      page,
      per_page: 10,
    },
  })
  const { mutate: modifyTitle } = usePostMrTitle()

  // const { mutate: approveMr, isPending: mrMergeIsPending } = usePostMrMerge(id)
  // const handleMrApprove = () => {
  //   approveMr(undefined, {
  //     onSuccess: () => {
  //       router.push(`/${scope}/mr`)
  //     }
  //   })
  // }

  const [_, setEditId] = useAtom(editIdAtom)
  const [refresh, setRefresh] = useAtom(refreshAtom)

  useEffect(() => {
    const load = async () => {
      await refetch()
      setEditId(FALSE_EDIT_VAL)
      setRefresh(0)
    }

    load()
  }, [refresh, refetch, setEditId, setRefresh])

  const { mutate: closeMr, isPending: mrCloseIsPending } = usePostMrClose(id)
  const handleMrClose = () => {
    if (closeHint === 'Close with comment') {
      send_comment()
    }
    closeMr(undefined, {
      onSuccess: () => {
        router.push(`/${scope}/mr`)
      }
    })
  }

  const { mutate: reopenMr, isPending: mrReopenIsPending } = usePostMrReopen(id)
  const handleMrReopen = () => {
    if (needComment.current) {
      send_comment()
      needComment.current = false
    }
    reopenMr(undefined, {
      onSuccess: () => {
        router.push(`/${scope}/mr`)
      }
    })
  }

  const { mutate: postMrComment, isPending: mrCommentIsPending } = usePostMrComment(id)

  const send_comment = () => {
    const currentContentHTML = editorRef.current?.editor?.getHTML() ?? '<p></p>'
    const issues = editorRef.current?.getLinkedIssues() || []

    /* eslint-disable-next-line no-console */
    console.log('commentIssues:', issues)

    if (trimHtml(currentContentHTML) === '') {
      toast.error('Please enter the content.')
    } else {
      postMrComment(
        { content: currentContentHTML },
        {
          onSuccess: () => {
            editorRef.current?.clearAndBlur()
          }
        }
      )
    }
  }

  const buttonClasses = 'cursor-pointer'
  const editorRef = useRef<SimpleNoteContentRef>(null)
  const onKeyDownScrollHandler = useHandleBottomScrollOffset({
    editor: editorRef.current?.editor
  })
  const { dropzone } = useUploadHelpers({
    upload: editorRef.current?.uploadAndAppendAttachments
  })

  const avatars = useAvatars()

  const memberMap = useMemberMap()

  const labels = useLabels()

  const labelMap = useLabelMap()

  const { open, handleAssignees, handleOpenChange, fetchSelected } = useAssigneesSelector({
    assignees: MrDetailData?.data?.assignees ?? [],
    assignRequest: (selected) =>
      mrAssignees(
        {
          data: {
            link: id,
            item_id: Number(item_id),
            assignees: selected
          }
        },
        {
          onSuccess: async () => {
            editorRef.current?.clearAndBlur()
            await refetch({ throwOnError: true })
          },
          onError: apiErrorToast
        }
      ),
    avatars
  })
  const {
    open: label_open,
    handleLabels,
    handleOpenChange: label_handleOpenChange,
    fetchSelected: label_fetchSelected
  } = useLabelsSelector({
    labelList: labels,
    labels: MrDetailData?.data?.labels ?? [],
    updateLabelsRequest: (selected) => {
      mrLabels(
        {
          data: {
            item_id: Number(item_id),
            label_ids: selected,
            link: `${tempId}`
          }
        },
        {
          onSuccess: async () => {
            editorRef.current?.clearAndBlur()
            await refetch({ throwOnError: true })
          },
          onError: apiErrorToast
        }
      )
    }
  })

  const handleSave = () => {
    if (editTitle === mrDetail?.title || editTitle === '') {
      apiErrorToast(new Error('Nothing Changed or is Empty'))
      return
    }
    setLoading(true)
    modifyTitle(
      { link: id, data: { content: editTitle as string } },
      {
        onError: (err) => apiErrorToast(err),
        onSuccess: async () => await refetch({ throwOnError: true }),
        onSettled: () => {
          setIsEdit(false)
          setLoading(false)
        }
      }
    )
  }

  const [tab] = useAtom(tabAtom)
  const Conversation = () => (
    <div className='flex gap-40'>
      <div className='mt-3 flex w-[60%] flex-col'>
        {detailIsLoading ? (
          <div className='flex h-16 items-center justify-center'>
            <LoadingSpinner />
          </div>
        ) : (
          mrDetail && <TimelineItems detail={mrDetail} id={id} type='mr' editorRef={editorRef} />
        )}
        <div style={{ marginTop: '12px' }} className='prose'>
          {/*<div className='flex'>*/}
          {/*  {mrDetail && mrDetail.status === 'open' && (*/}
          {/*    <Button*/}
          {/*      disabled={!login || mrMergeIsPending}*/}
          {/*      onClick={handleMrApprove}*/}
          {/*      aria-label='Merge MR'*/}
          {/*      className={cn(buttonClasses)}*/}
          {/*      loading={mrMergeIsPending}*/}
          {/*    >*/}
          {/*      Merge MR*/}
          {/*    </Button>*/}
          {/*  )}*/}
          {/*</div>*/}
          <div className='w-full'>
            {mrDetail && mrDetail.status === 'open' && (
              <MergeBox prId={id} />
            )}
          </div>
          <h2>Add a comment</h2>
          <input {...dropzone.getInputProps()} />
          <div className='rounded-lg border p-6'>
            <SimpleNoteContent
              commentId='temp' //  Temporary filling, replacement later
              ref={editorRef}
              editable='all'
              content={EMPTY_HTML}
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
          <div className='flex justify-end gap-2'>
            {mrDetail && mrDetail.status === 'open' && (
              <Button
                disabled={!login || mrCloseIsPending}
                onClick={handleMrClose}
                aria-label='Close Merge Request'
                className={cn(buttonClasses)}
                loading={mrCloseIsPending}
              >
                {closeHint}
              </Button>
            )}
            {mrDetail && mrDetail.status === 'closed' && (
              <Button
                disabled={!login || mrReopenIsPending}
                onClick={handleMrReopen}
                aria-label='Reopen Merge Request'
                className={cn(buttonClasses)}
                loading={mrReopenIsPending}
              >
                Reopen Merge Request
              </Button>
            )}
            <Button
              disabled={!login || mrCommentIsPending}
              onClick={() => send_comment()}
              aria-label='Comment'
              className={cn(buttonClasses)}
              loading={mrCommentIsPending}
            >
              Comment
            </Button>
          </div>
        </div>
      </div>
      {/* <SideBar /> */}
      <div className='flex flex-1 flex-col flex-wrap items-center'>
        <BadgeItem
          selectPannelProps={{ title: 'Assign up to 10 people to this issue' }}
          items={avatars}
          title='Assignees'
          handleGroup={(selected) => handleAssignees(selected)}
          open={open}
          // eslint-disable-next-line react-hooks/rules-of-hooks
          onOpenChange={(open) => handleOpenChange(open)}
          selected={fetchSelected}
        >
          {(el) => {
            const names = Array.from(new Set(splitFun(el)))

            return (
              <>
                {names.map((i, index) => (
                  // eslint-disable-next-line react/no-array-index-key
                  <div key={index} className='mb-4 flex items-center gap-2 px-4 text-sm text-gray-500'>
                    <MemberAvatar size='sm' member={memberMap.get(i)} />
                    <span>{i}</span>
                  </div>
                ))}
              </>
            )
          }}
        </BadgeItem>
        <BadgeItem
          selectPannelProps={{ title: 'Apply labels to this issue' }}
          items={labels}
          title='Labels'
          handleGroup={(selected) => handleLabels(selected)}
          open={label_open}
          onOpenChange={(open) => label_handleOpenChange(open)}
          selected={label_fetchSelected}
        >
          {(el) => {
            const names = splitFun(el)

            return (
              <>
                <div className='flex flex-wrap items-start px-4'>
                  {names.map((i, index) => {
                    const label = labelMap.get(i) ?? {}

                    return (
                      // eslint-disable-next-line react/no-array-index-key
                      <div key={index} className='mb-4 flex items-center justify-center pr-2'>
                        <div
                          className='rounded-full border px-2 text-sm text-[#fff]'
                          //eslint-disable-next-line react/forbid-dom-props
                          style={{ backgroundColor: label.color, borderColor: label.color }}
                        >
                          {label.name}
                        </div>
                      </div>
                    )
                  })}
                </div>
              </>
            )
          }}
        </BadgeItem>
        <BadgeItem title='Type' items={labels} />
        <BadgeItem title='Projects' items={labels} />
        <BadgeItem title='Milestones' items={labels} />
      </div>
    </div>
  )
  const FileChange = () => (
    <>
      {fileChgIsLoading ? (
        <div className='align-center container absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 justify-center'>
          <LoadingSpinner />
        </div>
        ) : MrFilesChangedData?.data?.page.items ? (
          <FileDiff diffs={MrFilesChangedData.data.page.items} treeData={MrFilesChangedData.data} />
        ) : (
        <div>No files changed</div>
      )}
    </>
  )

  return (
    <ThemeProvider>
      <BaseStyles>
        <div className='h-screen overflow-auto p-6'>
          <div className='mb-2 w-[70%]'>
            {!isEdit && (
              <>
                <div className='flex w-full items-center justify-between'>
                  <UIText size='text-2xl' weight='font-bold' className='-tracking-[1px] lg:flex'>
                    {`${mrDetail?.title || ''}`}
                    <span>&nbsp;</span>
                    <span className='font-light !text-[#59636e]'>${id}</span>
                  </UIText>
                  <Button
                    onClick={() => {
                      setEditTitle(mrDetail?.title)
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
                    onChange={(e) => setEditTitle(e.target.value)}
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
          <div>
            <TabLayout>
              {tab === 'check' && <Checks mr={id} />}
              {tab === 'conversation' && <Conversation />}
              {tab === 'filechange' && <FileChange />}
            </TabLayout>
          </div>
        </div>
      </BaseStyles>
    </ThemeProvider>
  )
}

MRDetailPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default MRDetailPage
