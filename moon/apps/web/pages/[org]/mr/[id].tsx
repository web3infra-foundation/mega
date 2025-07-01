'use client'

import React, { useRef, useState } from 'react';
import { PicturePlusIcon} from '@gitmono/ui/Icons'
import { useRouter } from 'next/router';
import FileDiff from '@/components/DiffView/FileDiff';
import { Button, LoadingSpinner, UIText } from '@gitmono/ui';
import { cn } from '@gitmono/ui/utils';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import { AppLayout } from '@/components/Layout/AppLayout';
import { PageWithLayout } from '@/utils/types';
import { useGetMrDetail } from '@/hooks/useGetMrDetail'
import { useGetMrFilesChanged } from '@/hooks/useGetMrFilesChanged';
import { usePostMrComment } from '@/hooks/usePostMrComment'
import { usePostMrMerge } from '@/hooks/usePostMrMerge'
import { usePostMrReopen } from '@/hooks/usePostMrReopen';
import { usePostMrClose } from '@/hooks/usePostMrClose';
import { useScope } from '@/contexts/scope'
import { SimpleNoteContent, SimpleNoteContentRef } from '@/components/SimpleNoteEditor/SimpleNoteContent';
import { EMPTY_HTML } from '@/atoms/markdown'
import { useHandleBottomScrollOffset } from '@/components/NoteEditor/useHandleBottomScrollOffset'
import { trimHtml } from '@/utils/trimHtml'
import { toast } from 'react-hot-toast'
import { ComposerReactionPicker } from '@/components/Reactions/ComposerReactionPicker';
import { useUploadHelpers } from '@/hooks/useUploadHelpers';
import { CommentDiscussionIcon, FileDiffIcon } from '@primer/octicons-react'
import TimelineItems from '@/components/MrView/TimelineItems';

export interface MRDetail {
    status: string,
    conversations: Conversation[],
    title: string,
}
export interface Conversation {
    id: number,
    user_id: number,
    conv_type: string,
    comment: string,
    created_at: number,
}

const  MRDetailPage:PageWithLayout<any> = () =>{
    const router = useRouter();
    const { id : tempId } = router.query;
    const { scope } = useScope()
    const [login, _setLogin] = useState(true);
    const [isReactionPickerOpen, setIsReactionPickerOpen] = useState(false)
    const id = typeof tempId === 'string' ? tempId : '';
    const { data: MrDetailData, isLoading: detailIsLoading } = useGetMrDetail(id)
    const mrDetail = MrDetailData?.data as MRDetail | undefined
    const UnderlinePanels = require('@primer/react/experimental')
    
    if (mrDetail && typeof mrDetail.status === 'string') {
      mrDetail.status = mrDetail.status.toLowerCase();
    }

    const { data: MrFilesChangedData, isLoading: fileChgIsLoading} = useGetMrFilesChanged(id)

    const { mutate: approveMr, isPending : mrMergeIsPending } = usePostMrMerge(id)
    const handleMrApprove = () => {
      approveMr(undefined, {
        onSuccess: () => {
          router.push(`/${scope}/mr`)
        },
      })
    }

    const { mutate: closeMr, isPending: mrCloseIsPending } = usePostMrClose(id)
    const handleMrClose = () => {
      closeMr(undefined, {
        onSuccess: () => {
          router.push(`/${scope}/mr`)
        },
      })
    }

    const { mutate: reopenMr, isPending: mrReopenIsPending } = usePostMrReopen(id)
    const handleMrReopen = () => {
      reopenMr(undefined,{
        onSuccess: () => {
          router.push(`/${scope}/mr`)
        },
      })
    }

    const { mutate: postMrComment, isPending : mrCommentIsPending } = usePostMrComment(id)

    const send_comment = () => {
      const currentContentHTML = editorRef.current?.editor?.getHTML() ?? '<p></p>';

     if (trimHtml(currentContentHTML) === '') {
        toast.error('Please enter the content.')
     }else {
        postMrComment(
        { content: currentContentHTML },
        {
          onSuccess: () =>{
            editorRef.current?.clearAndBlur()
          }
        }
      );
     }
    }

    const buttonClasses= 'cursor-pointer';
    const editorRef = useRef<SimpleNoteContentRef>(null);
    const onKeyDownScrollHandler = useHandleBottomScrollOffset({
      editor: editorRef.current?.editor
    })
    const { dropzone } = useUploadHelpers({
      upload: editorRef.current?.uploadAndAppendAttachments
    })

  return (
    <div className='h-screen overflow-auto p-6'>
      <div className='mb-2'>
        <UIText size='text-2xl' weight='font-bold' className='-tracking-[1px] lg:flex'>
          {`${mrDetail?.title || ''}#${id}`}
        </UIText>
      </div>
      <div>
        <UnderlinePanels aria-label='Select a tab'>
          <UnderlinePanels.Tab icon={CommentDiscussionIcon}>Conversation</UnderlinePanels.Tab>
          <UnderlinePanels.Tab icon={FileDiffIcon}>Files Changed</UnderlinePanels.Tab>
          <UnderlinePanels.Panel>
            <div className="flex flex-col w-full mt-3">
              {detailIsLoading ? (
                <div className='flex items-center justify-center'>
                  <LoadingSpinner />
                </div>
                ) : (
                  <TimelineItems mrDetail={mrDetail} id={id} />
              )}
              <div className='prose mt-3'>
                <div className="flex">
                  {mrDetail && mrDetail.status === "open" &&
                    <Button
                      disabled={!login || mrMergeIsPending}
                      onClick={handleMrApprove}
                      aria-label="Merge MR"
                      className={cn(buttonClasses)}
                      loading={mrMergeIsPending}
                    >
                      Merge MR
                    </Button>
                  }
                </div>
                <h2>Add a comment</h2>
                <input {...dropzone.getInputProps()} />
                <div className='border p-6 rounded-lg'>
                  <SimpleNoteContent
                    commentId="temp" //  Temporary filling, replacement later
                    ref={editorRef}
                    editable="all"
                    content={EMPTY_HTML}
                    autofocus={true}
                    onKeyDown={onKeyDownScrollHandler}
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
                <div className="flex gap-2 justify-end">
                  {mrDetail && mrDetail.status === "open" &&
                    <Button
                      disabled={!login || mrCloseIsPending}
                      onClick={handleMrClose}
                      aria-label="Close Merge Request"
                      className={cn(buttonClasses)}
                      loading={mrCloseIsPending}
                    >
                      Close Merge Request
                    </Button>
                  }
                  {mrDetail && mrDetail.status === "closed" &&
                    <Button
                      disabled={!login || mrReopenIsPending}
                      onClick={handleMrReopen}
                      aria-label="Reopen Merge Request"
                      className={cn(buttonClasses)}
                      loading={mrReopenIsPending}
                    >
                      Reopen Merge Request
                    </Button>
                  }
                  <Button
                    disabled={!login || mrCommentIsPending}
                    onClick={() => send_comment()}
                    aria-label="Comment"
                    className={cn(buttonClasses)}
                    loading={mrCommentIsPending}
                  >
                    Comment
                  </Button>
                </div>
              </div>
            </div>
          </UnderlinePanels.Panel>
          <UnderlinePanels.Panel>
            {fileChgIsLoading ? (
              <div className='flex items-center justify-center'>
                <LoadingSpinner />
              </div>
            ) : MrFilesChangedData?.data?.content ? (
              <FileDiff diffs={MrFilesChangedData.data.content} />
            ) : (
              <div>No files changed</div>
            )}
          </UnderlinePanels.Panel>
        </UnderlinePanels>
      </div>
    </div>
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
