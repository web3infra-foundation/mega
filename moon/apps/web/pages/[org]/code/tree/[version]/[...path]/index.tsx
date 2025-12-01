'use client'

import React, { useMemo, useState } from 'react'
import { Theme } from '@radix-ui/themes'
import { useParams } from 'next/navigation'

import { CommonResultVecTreeCommitItem } from '@gitmono/types/generated'
import { Button } from '@gitmono/ui/Button'
import { CloseIcon } from '@gitmono/ui/Icons'

import CodeTable from '@/components/CodeView/CodeTable'
import CommitHistory from '@/components/CodeView/CommitHistory'
import NewCodeView from '@/components/CodeView/NewCodeView/NewCodeView'
import TagSwitcher from '@/components/CodeView/Tags/TagSwitcher'
import BreadCrumb from '@/components/CodeView/TreeView/BreadCrumb'
import CloneTabs from '@/components/CodeView/TreeView/CloneTabs'
import RepoTree from '@/components/CodeView/TreeView/RepoTree'
import SyncRepoButton from '@/components/CodeView/TreeView/SyncRepoButton'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useGetBlob } from '@/hooks/useGetBlob'
import { useGetTreeCommitInfo } from '@/hooks/useGetTreeCommitInfo'
import { useGetTreePathCanClone } from '@/hooks/useGetTreePathCanClone'

function TreeDetailPage() {
  const params = useParams()
  const { version, path = [] } = params as { version: string; path?: string[] }

  const refs = version === 'main' ? undefined : version
  const new_path = '/' + path?.join('/')

  const [newPath, setNewPath] = useState(new_path)
  const { data: TreeCommitInfo } = useGetTreeCommitInfo(newPath, refs)

  type DirectoryType = NonNullable<CommonResultVecTreeCommitItem['data']>
  const directory: DirectoryType = useMemo(() => TreeCommitInfo?.data ?? [], [TreeCommitInfo])

  const { data: canClone } = useGetTreePathCanClone({ path: newPath })

  const reqPath = `${new_path}/README.md`
  const { data: readmeContent } = useGetBlob({ path: reqPath, refs })

  const [isNewCode, setIsNewCode] = useState(false)
  const [newEntryType, setNewEntryType] = useState<'file' | 'folder'>('file')

  const treeStyle = {
    borderRadius: 8,
    width: '20%',
    minWidth: '300px',
    flexShrink: 0,
    background: '#fff',
    height: 'calc(100vh - 96px)',
    overflow: 'auto',
    paddingRight: '8px'
  }

  const codeStyle = {
    borderRadius: 8,
    width: 'calc(80% - 8px)',
    background: '#fff',
    height: 'calc(100vh - 96px)',
    overflow: 'auto',
    paddingRight: '8px'
  }

  const handleNewClick = (type: 'file' | 'folder') => {
    setNewEntryType(type)
    setIsNewCode(true)
  }
  const handleCloseClick = () => {
    setIsNewCode(false)
  }

  return (
    <Theme>
      <div className='relative m-4 h-screen'>
        <div className='flex min-h-12 items-center justify-between'>
          <BreadCrumb path={path} />
          {!isNewCode ? (
            <>
              <div className='m-1 flex justify-end gap-2'>
                <TagSwitcher />
                {version == 'main' && path[0] !== 'third-party' && (
                  <Button onClick={() => handleNewClick('file')}>New File</Button>
                )}
                {version == 'main' && path[0] !== 'third-party' && (
                  <Button onClick={() => handleNewClick('folder')}>New Folder</Button>
                )}
                {canClone?.data && <CloneTabs />}
                {path[0] === 'third-party' && version == 'main' && <SyncRepoButton currentPath={newPath} />}
              </div>
            </>
          ) : (
            <Button onClick={handleCloseClick}>
              <CloseIcon />
            </Button>
          )}
        </div>
        <div className='flex h-full gap-4'>
          <div style={treeStyle}>
            <RepoTree onCommitInfoChange={(path: string) => setNewPath(path)} />
          </div>

          {!isNewCode ? (
            <div style={codeStyle}>
              <div>
                <CommitHistory flag={'contents'} path={newPath} refs={refs} />
              </div>
              <CodeTable directory={directory} loading={!TreeCommitInfo} readmeContent={readmeContent?.data} />
            </div>
          ) : (
            <div className='pb-18 flex-1 overflow-hidden'>
              <NewCodeView
                currentPath={new_path}
                onClose={handleCloseClick}
                defaultType={newEntryType}
                version={version}
              />
            </div>
          )}
        </div>
      </div>
    </Theme>
  )
}

TreeDetailPage.getProviders = (
  page:
    | string
    | number
    | boolean
    | React.ReactElement<any, string | React.JSXElementConstructor<any>>
    | Iterable<React.ReactNode>
    | React.ReactPortal
    | Promise<React.AwaitedReactNode>
    | null
    | undefined,
  pageProps: React.JSX.IntrinsicAttributes & { children?: React.ReactNode | undefined }
) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default TreeDetailPage
