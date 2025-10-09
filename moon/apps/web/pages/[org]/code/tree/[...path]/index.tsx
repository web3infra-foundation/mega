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
import BreadCrumb from '@/components/CodeView/TreeView/BreadCrumb'
import CloneTabs from '@/components/CodeView/TreeView/CloneTabs'
import RepoTree from '@/components/CodeView/TreeView/RepoTree'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import TagSwitcher from '@/components/CodeView/Tags/TagSwitcher'
import { useGetBlob } from '@/hooks/useGetBlob'
import { useGetTreeCommitInfo } from '@/hooks/useGetTreeCommitInfo'
import { useGetTreePathCanClone } from '@/hooks/useGetTreePathCanClone'

function TreeDetailPage() {
  const params = useParams()
  const { path = [] } = params as { path?: string[] }
  const new_path = '/' + path?.join('/')

  const [newPath, setNewPath] = useState(new_path)
  const { data: TreeCommitInfo } = useGetTreeCommitInfo(newPath)

  type DirectoryType = NonNullable<CommonResultVecTreeCommitItem['data']>
  const directory: DirectoryType = useMemo(() => TreeCommitInfo?.data ?? [], [TreeCommitInfo])

  const { data: canClone } = useGetTreePathCanClone({ path: newPath })

  const reqPath = `${new_path}/README.md`
  const { data: readmeContent } = useGetBlob({ path: reqPath })

  const [isNewCode, setIsNewCode] = useState(false)

  const commitInfo = {
    user: {
      avatar_url: 'https://avatars.githubusercontent.com/u/112836202?v=4&size=40',
      name: 'yetianxing2014'
    },
    message: '[feat(libra)]: 为 config 命令添加 --default参数 (#1119)',
    hash: '5fe4235',
    date: '3 months ago'
  }

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

  const handleNewClick = () => {
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
                <Button onClick={handleNewClick}>New</Button>
                {canClone?.data && <CloneTabs />}
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
              {commitInfo && (
                <div>
                  <CommitHistory flag={'contents'} info={commitInfo} />
                </div>
              )}
              <CodeTable
                directory={directory}
                loading={!TreeCommitInfo}
                onCommitInfoChange={(path: string) => setNewPath(path)}
                readmeContent={readmeContent?.data}
              />
            </div>
          ) : (
            <div className='pb-18 flex-1 overflow-hidden'>
              <NewCodeView />
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
