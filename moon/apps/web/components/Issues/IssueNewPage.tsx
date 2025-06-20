'use client'

import { useCallback, useState, useRef } from 'react'
import { Flex, Input, Space } from 'antd/lib'
import { useRouter } from 'next/router'
import toast from 'react-hot-toast'

import { Button, LargeTitle, PicturePlusIcon } from '@gitmono/ui'
import { usePostIssueSubmit } from '@/hooks/issues/usePostIssueSubmit'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { SimpleNoteContent, SimpleNoteContentRef } from '@/components/SimpleNoteEditor/SimpleNoteContent';
import { EMPTY_HTML } from '@/atoms/markdown'
import { useHandleBottomScrollOffset } from '@/components/NoteEditor/useHandleBottomScrollOffset'
import { useUploadHelpers } from '@/hooks/useUploadHelpers';
import { ComposerReactionPicker } from '@/components/Reactions/ComposerReactionPicker';
import { trimHtml } from '@/utils/trimHtml'

export default function IssueNewPage() {
  const [title, setTitle] = useState('')
  const [loadings, setLoadings] = useState<boolean[]>([])
  const router = useRouter()
  const { mutate: submitNewIssue } = usePostIssueSubmit()
  const [isReactionPickerOpen, setIsReactionPickerOpen] = useState(false)
  const set_to_loading = (index: number) => {
    setLoadings((prevLoadings) => {
      const newLoadings = [...prevLoadings]

      newLoadings[index] = true
      return newLoadings
    })
  }

  const cancel_loading = (index: number) => {
    setLoadings((prevLoadings) => {
      const newLoadings = [...prevLoadings]

      newLoadings[index] = false
      return newLoadings
    })
  }

  const submit = useCallback(
    () => {
      const currentContentHTML = editorRef.current?.editor?.getHTML() ?? '<p></p>';

      if (trimHtml(currentContentHTML) === '' || !title) {
        toast.error('please fill the issue list first!')
        return
      }
      set_to_loading(3)
      submitNewIssue(
        { data: { title, description: currentContentHTML } },
        {
          onSuccess: (_response) => {
            editorRef.current?.clearAndBlur()
            cancel_loading(3)
            toast.success('success')
            router.push(`/${router.query.org}/issue`)
          },
          onError: () => apiErrorToast
        }
      )
    },
    [router, title, submitNewIssue]
  )

  const editorRef = useRef<SimpleNoteContentRef>(null);
  const onKeyDownScrollHandler = useHandleBottomScrollOffset({
    editor: editorRef.current?.editor
  })
  const { dropzone } = useUploadHelpers({
    upload: editorRef.current?.uploadAndAppendAttachments
  })

  return (
    <>
      <div className='container p-10'>
        <LargeTitle>Add a title</LargeTitle>
        <Space direction='vertical' style={{ width: '100%' }}>
          <h1>
            Add a title
            <Input
              aria-label='title'
              name='title'
              placeholder='Title'
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            ></Input>
          </h1>
        </Space>
        <Space direction='vertical' style={{ width: '100%' }}>
        <div className="prose flex flex-col w-full mt-2">
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
        </div>
          <Flex justify={'flex-end'}>
            <Button
              type={'submit'}
              loading={loadings[3]}
              disabled={!title}
              onClick={() => submit()}
            >
              Submit New Issue
            </Button>
          </Flex>
        </Space>
      </div>
    </>
  )
}
