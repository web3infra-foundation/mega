'use client'

import React, { useCallback, useMemo, useRef, useState } from 'react'
import { DiffFile, DiffModeEnum, DiffView } from '@git-diff-view/react'
import toast from 'react-hot-toast'

import '@git-diff-view/react/styles/diff-view.css'

import { Button } from '@gitmono/ui/Button'
import { Dialog } from '@gitmono/ui/Dialog'

import { useDiffPreview } from '@/hooks/useDiffPreview'
import { useGetCurrentUser } from '@/hooks/useGetCurrentUser'
import { useUpdateBlob } from '@/hooks/useUpdateBlob'
import { getLangFromFileName } from '@/utils/getLanguageDetection'

interface BlobEditorProps {
  fileContent: string
  filePath: string
  fileName: string
  onCancel: () => void
}

type ViewMode = 'edit' | 'preview'

export default function BlobEditor({ fileContent, filePath, fileName, onCancel }: BlobEditorProps) {
  const { data: currentUser } = useGetCurrentUser()

  const updateBlobMutation = useUpdateBlob()
  const diffPreviewMutation = useDiffPreview()

  const [content, setContent] = useState(fileContent)

  const [editedFileName, setEditedFileName] = useState(fileName)
  const [commitMessage, setCommitMessage] = useState(`Update ${fileName}`)
  const [commitDescription, setCommitDescription] = useState('')

  const [viewMode, setViewMode] = useState<ViewMode>('edit')

  const [diffResult, setDiffResult] = useState<any>(null)
  const [diffFile, setDiffFile] = useState<DiffFile | null>(null)

  const [isCommitDialogOpen, setIsCommitDialogOpen] = useState(false)

  const lineNumbersRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const contentLines = useMemo(() => content.split('\n'), [content])

  const hasChanges = useMemo(
    () => content !== fileContent || editedFileName !== fileName,
    [content, fileContent, editedFileName, fileName]
  )

  const pathSegments = useMemo(() => {
    const segments = filePath.split('/').filter(Boolean)

    return segments.slice(0, -1)
  }, [filePath])

  const fullEditedPath = useMemo(() => {
    const dir = pathSegments.join('/')

    return dir ? `${dir}/${editedFileName}` : editedFileName
  }, [pathSegments, editedFileName])

  const detectedLanguage = useMemo(() => getLangFromFileName(editedFileName), [editedFileName])

  const handlePreviewClick = useCallback(async () => {
    setViewMode('preview')

    if (!hasChanges) {
      return
    }

    if (!diffResult) {
      try {
        const result = await diffPreviewMutation.mutateAsync({
          path: filePath,
          content: content
        })

        setDiffResult(result)

        if (result?.data?.data) {
          const diff = new DiffFile('', '', '', '', [result.data.data], detectedLanguage || 'plaintext')

          diff.init()
          diff.buildSplitDiffLines()
          diff.buildUnifiedDiffLines()
          setDiffFile(diff)
        }
      } catch (error: any) {
        toast.error(error?.message)
      }
    }
  }, [content, filePath, hasChanges, diffPreviewMutation, diffResult, detectedLanguage])

  const handleCommitClick = useCallback(() => {
    if (!hasChanges) {
      return
    }
    setIsCommitDialogOpen(true)
  }, [hasChanges])

  const handleSave = useCallback(async () => {
    await updateBlobMutation.mutateAsync({
      path: fullEditedPath,
      content: content,
      commit_message: commitDescription ? `${commitMessage}\n\n${commitDescription}` : commitMessage,
      author_email: currentUser?.email,
      author_username: currentUser?.username
    })

    setIsCommitDialogOpen(false)
    onCancel()
  }, [
    updateBlobMutation,
    fullEditedPath,
    content,
    commitDescription,
    commitMessage,
    currentUser?.email,
    currentUser?.username,
    onCancel
  ])

  const handleTextareaChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setContent(e.target.value)

    setDiffResult(null)
    setDiffFile(null)
  }

  const handleScroll = useCallback(() => {
    if (textareaRef.current && lineNumbersRef.current) {
      lineNumbersRef.current.scrollTop = textareaRef.current.scrollTop
    }
  }, [])

  const renderEditView = () => {
    return (
      <div className='flex h-full w-full overflow-hidden font-mono text-sm leading-6'>
        <div
          ref={lineNumbersRef}
          className='h-full select-none overflow-hidden border-r border-gray-200 bg-gray-50 px-4 text-right text-gray-400'
          style={{ flexShrink: 0 }}
        >
          {contentLines.map((_, index) => (
            // eslint-disable-next-line react/no-array-index-key
            <div key={index} className='leading-6' style={{ height: '1.5rem' }}>
              {index + 1}
            </div>
          ))}
        </div>

        <div className='flex-1 overflow-hidden'>
          <textarea
            ref={textareaRef}
            value={content}
            onChange={handleTextareaChange}
            onScroll={handleScroll}
            className='h-full w-full resize-none overflow-auto border-0 bg-transparent p-0 pl-4 font-mono text-sm leading-6 focus:outline-none'
            spellCheck={false}
            style={{ tabSize: 2 }}
          />
        </div>
      </div>
    )
  }

  const renderPreviewView = () => {
    if (!hasChanges) {
      return (
        <div className='flex h-full items-center justify-center text-gray-500'>
          <div className='text-center'>
            <p className='text-lg font-medium'>No changes</p>
            <p className='mt-2 text-sm'>Please edit the file content first</p>
          </div>
        </div>
      )
    }

    if (diffPreviewMutation.isPending) {
      return (
        <div className='flex h-full items-center justify-center text-gray-500'>
          <div className='text-center'>
            <p className='text-lg font-medium'>Loading...</p>
            <p className='mt-2 text-sm'>Generating diff preview</p>
          </div>
        </div>
      )
    }

    if (!diffFile) {
      return (
        <div className='flex h-full items-center justify-center text-gray-500'>
          <div className='text-center'>
            <p className='text-lg font-medium'>Failed to load diff preview</p>
            <p className='mt-2 text-sm'>Please try again</p>
          </div>
        </div>
      )
    }

    return (
      <div className='h-full overflow-auto'>
        <style>{`

          .diff-line-code-content {
            min-height: 1.4em;
            white-space: pre !important;
          }
          .diff-line-code-content:empty::before {
            content: '\\200b';
            display: inline;
          }

          .diff-line-old-text,
          .diff-line-new-text {
            white-space: pre !important;
          }
        `}</style>
        <DiffView
          diffFile={diffFile}
          diffViewFontSize={14}
          diffViewHighlight
          diffViewMode={DiffModeEnum.Split}
          diffViewWrap
        />
      </div>
    )
  }

  return (
    <div className='flex min-h-0 w-full flex-1 flex-col gap-2'>
      <div className='flex min-h-14 w-full flex-shrink-0 items-center justify-between px-2'>
        <div className='flex max-w-[900px] flex-wrap items-center gap-x-1 gap-y-2 text-gray-700'>
          {pathSegments.map((seg, i) => (
            // eslint-disable-next-line react/no-array-index-key
            <React.Fragment key={i}>
              <span className='font-medium text-blue-600'>{seg}</span>
              <span>/</span>
            </React.Fragment>
          ))}

          <input
            type='text'
            value={editedFileName}
            onChange={(e) => setEditedFileName(e.target.value)}
            placeholder='fileName'
            className='min-w-[180px] rounded border border-gray-300 px-2 py-1 text-sm font-medium text-gray-900 outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-500'
            disabled={updateBlobMutation.isPending}
          />
        </div>

        <div className='flex gap-2'>
          <Button variant='flat' onClick={onCancel} disabled={updateBlobMutation.isPending}>
            Cancel changes
          </Button>
          <Button onClick={handleCommitClick} disabled={updateBlobMutation.isPending || !hasChanges}>
            Commit changes
          </Button>
        </div>
      </div>

      <div className='flex min-h-0 w-full flex-1 flex-col rounded-xl border border-[#bec7ce]'>
        <div className='flex h-14 w-full flex-shrink-0 items-center rounded-t-xl border-b border-[#d0d9e0] bg-[#f9fbfd] px-4'>
          <div className='inline-flex rounded-md border border-gray-300 bg-white'>
            <button
              onClick={() => setViewMode('edit')}
              className={`rounded-l-md px-4 py-2 text-sm font-medium ${
                viewMode === 'edit' ? 'bg-gray-100 text-gray-900' : 'bg-white text-gray-500 hover:text-gray-700'
              }`}
            >
              Edit
            </button>
            <button
              onClick={handlePreviewClick}
              className={`rounded-r-md px-4 py-2 text-sm font-medium ${
                viewMode === 'preview' ? 'bg-gray-100 text-gray-900' : 'bg-white text-gray-500 hover:text-gray-700'
              }`}
            >
              Preview
            </button>
          </div>
        </div>

        <div className='min-h-0 flex-1 overflow-hidden'>
          {viewMode === 'edit' && renderEditView()}
          {viewMode === 'preview' && renderPreviewView()}
        </div>
      </div>

      <Dialog.Root open={isCommitDialogOpen} onOpenChange={setIsCommitDialogOpen}>
        <Dialog.Content>
          <Dialog.CloseButton />
          <Dialog.Header>
            <Dialog.Title>Commit changes</Dialog.Title>
          </Dialog.Header>

          <div className='flex flex-col gap-4 py-4'>
            <div className='flex flex-col gap-2'>
              <label className='text-sm font-medium text-gray-700'>Commit message *</label>
              <input
                type='text'
                value={commitMessage}
                onChange={(e) => setCommitMessage(e.target.value)}
                placeholder={`update ${fileName}`}
                className='w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500'
                disabled={updateBlobMutation.isPending}
              />
            </div>

            <div className='flex flex-col gap-2'>
              <label className='text-sm font-medium text-gray-700'>Extended description (optional)</label>
              <textarea
                value={commitDescription}
                onChange={(e) => setCommitDescription(e.target.value)}
                placeholder='Add an optional extended description...'
                rows={4}
                className='w-full resize-none rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500'
                disabled={updateBlobMutation.isPending}
              />
            </div>
          </div>

          <Dialog.Footer>
            <Dialog.TrailingActions>
              <Button
                variant='flat'
                onClick={() => setIsCommitDialogOpen(false)}
                disabled={updateBlobMutation.isPending}
              >
                Cancel
              </Button>
              <Button onClick={handleSave} disabled={updateBlobMutation.isPending || !commitMessage.trim()}>
                {updateBlobMutation.isPending ? 'Submitting...' : 'Confirm submission'}
              </Button>
            </Dialog.TrailingActions>
          </Dialog.Footer>
        </Dialog.Content>
      </Dialog.Root>
    </div>
  )
}
