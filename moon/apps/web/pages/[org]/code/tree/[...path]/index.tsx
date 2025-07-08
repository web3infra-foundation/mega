'use client'

import React, { useMemo, useState } from 'react'
import { Theme } from '@radix-ui/themes'
import { Flex, Layout } from 'antd'
import { useParams } from 'next/navigation'
import { CommonResultVecTreeCommitItem } from '@gitmono/types/generated'
import CodeTable from '@/components/CodeView/CodeTable'
import BreadCrumb from '@/components/CodeView/TreeView/BreadCrumb'
import CloneTabs from '@/components/CodeView/TreeView/CloneTabs'
import RepoTree from '@/components/CodeView/TreeView/RepoTree'
import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { useGetTreeCommitInfo } from '@/hooks/useGetTreeCommitInfo'
import { useGetTreePathCanClone } from '@/hooks/useGetTreePathCanClone'
import CommitHistory from '@/components/CodeView/CommitHistory'
import { useGetBlob } from '@/hooks/useGetBlob'

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
  const  {data: readmeContent}=useGetBlob({path:reqPath})


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
    overflow: 'hidden',
    width: 'calc(20% - 8px)',
    minWidth: 'calc(20% - 8px)',
    background: '#fff'
  }

  const codeStyle = {
    borderRadius: 8,
    overflow: 'hidden',
    width: 'calc(80% - 8px)',
    height:'100%',
    background: '#fff'
  }

  const breadStyle = {
    minHeight: 30,
    borderRadius: 8,
    overflow: 'hidden',
    width: 'calc(100% - 8px)',
    background: '#fff'
  }

  return (
    <div className='relative m-2 h-screen overflow-auto'>

        <Flex gap='middle' wrap>
          <Layout style={breadStyle}>
            <BreadCrumb path={path} />
            {canClone?.data && (
              <Flex justify={'flex-end'} className='m-1'>
                <CloneTabs/>
              </Flex>
            )}
          </Layout>
          {/* tree */}
          <Layout style={treeStyle}>
            <RepoTree 
            flag={'contents'}
            onCommitInfoChange={(path:string)=>setNewPath(path)} />
          </Layout>

          <Layout style={codeStyle}>
            {
             commitInfo &&  <Layout>
                <CommitHistory flag={'contents'} info={commitInfo}/>
              </Layout>
            }
          <CodeTable 
          directory={directory} 
          loading={!TreeCommitInfo} 
          onCommitInfoChange={(path:string)=>setNewPath(path)}
          readmeContent={readmeContent?.data}
          />
        
         </Layout>
        </Flex>

    </div>
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
      <Theme>
        <AppLayout {...pageProps}>{page}</AppLayout>
      </Theme>
    </AuthAppProviders>
  )
}

export default TreeDetailPage
