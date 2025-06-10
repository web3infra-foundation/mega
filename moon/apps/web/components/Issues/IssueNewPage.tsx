'use client'

import { useCallback, useState } from 'react'
import { Flex, Input, Space } from 'antd/lib'
import { useRouter } from 'next/router'
import toast from 'react-hot-toast'

import { Button, LargeTitle } from '@gitmono/ui'

import RichEditor from '@/components/MrView/rich-editor/RichEditor'
import { usePostIssueSubmit } from '@/hooks/issues/usePostIssueSubmit'
import { apiErrorToast } from '@/utils/apiErrorToast'

export default function IssueNewPage() {
  const [editorState, setEditorState] = useState('')
  const [title, setTitle] = useState('')
  const [loadings, setLoadings] = useState<boolean[]>([])
  const router = useRouter()
  const { mutate: submitNewIssue } = usePostIssueSubmit()
  const [editorHasText, setEditorHasText] = useState(false)
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
    (description: string) => {
      if (JSON.parse(description).root.children[0].children.length === 0 || !title) {
        toast.error('please fill the issue list first!')
        return
      }
      set_to_loading(3)
      submitNewIssue(
        { data: { title, description } },
        {
          onSuccess: (_response) => {
            setEditorState('')
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
          <h1>Add a description</h1>
          <RichEditor setEditorState={setEditorState} setEditorHasText={setEditorHasText} />
          <Flex justify={'flex-end'}>
            <Button type={'submit'} disabled={!editorHasText} loading={loadings[3]} onClick={() => submit(editorState)}>
              Submit New Issue
            </Button>
          </Flex>
        </Space>
      </div>
    </>
  )
}
