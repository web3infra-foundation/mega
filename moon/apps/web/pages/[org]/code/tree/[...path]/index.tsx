'use client'

import CodeTable from '@/components/CodeView/CodeTable';
import Bread from '@/components/CodeView/TreeView/BreadCrumb';
import RepoTree from '@/components/CodeView/TreeView/RepoTree';
import CloneTabs from '@/components/CodeView/TreeView/CloneTabs';
import React, {useEffect, useMemo, useState} from 'react';
import {Flex, Layout} from 'antd'
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import { AppLayout } from '@/components/Layout/AppLayout';
import { CommonResultVecTreeCommitItem } from '@gitmono/types/generated';
import { useGetTreeCommitInfo } from '@/hooks/useGetTreeCommitInfo';
import { useParams } from 'next/navigation';
import { useGetTreePathCanClone } from '@/hooks/useGetTreePathCanClone';

function TreeDetailPage() {
  const params = useParams();
  const { path = [] } = params as { path?: string[] };
  const new_path = '/' + path?.join('/');

  const { data:TreeCommitInfo } = useGetTreeCommitInfo(new_path)

  type DirectoryType = NonNullable<CommonResultVecTreeCommitItem['data']>;
  const directory:DirectoryType = useMemo(() => TreeCommitInfo?.data ?? [], [TreeCommitInfo]);
  
  const { data: canClone } = useGetTreePathCanClone({ path: new_path })
  const [readmeContent, setReadmeContent] = useState("");
  const [endpoint, setEndPoint] = useState("");
  

  useEffect(() => {
      const fetchData = async () => {
          try {
              const readmeContent = await getReadmeContent(new_path, directory);

              setReadmeContent(readmeContent);
              const endpoint = await getEndpoint();

              setEndPoint(endpoint);
          } catch (error) {
              // eslint-disable-next-line no-console
              console.error('Error fetching data:', error);
          }
      };

      fetchData();
  }, [new_path,directory]);

  const treeStyle = {
      borderRadius: 8,
      overflow: 'hidden',
      width: 'calc(20% - 8px)',
      maxWidth: 'calc(20% - 8px)',
      background: '#fff',
  };

  const codeStyle = {
      borderRadius: 8,
      overflow: 'hidden',
      width: 'calc(80% - 8px)',
      background: '#fff',
  };

  const breadStyle = {
      minHeight: 30,
      borderRadius: 8,
      overflow: 'hidden',
      width: 'calc(100% - 8px)',
      background: '#fff',
  };

  return (
    <div className='m-2'>
      <Flex gap="middle" wrap>
          <Layout style={breadStyle}>
              <Bread path={path} />
              {
                  canClone?.data &&
                  <Flex justify={'flex-end'} >
                      <CloneTabs endpoint={endpoint} />
                  </Flex>
              }
          </Layout>
            {/* tree */}
          <Layout style={treeStyle}>
              <RepoTree directory={directory} />
          </Layout>
          <Layout style={codeStyle}>
              <CodeTable directory={directory} readmeContent={readmeContent} />
          </Layout>
      </Flex>
    </div>
  );
}

async function getReadmeContent(pathname: string, directory: any) {
  let readmeContent = '';

  for (const project of directory || []) {
      if (project.name === 'README.md' && project.content_type === 'file') {
          const res = await fetch(`/api/blob?path=${pathname}/README.md`);
          const response = await res.json();

          readmeContent = response.data.data;
          break;
      }
  }
  return readmeContent
}


async function getEndpoint() {
  const res = await fetch(`/host`);
  const response = await res.json();

  return response.endpoint
}

TreeDetailPage.getProviders = (page: string | number | boolean | React.ReactElement<any, string | React.JSXElementConstructor<any>> | Iterable<React.ReactNode> | React.ReactPortal | Promise<React.AwaitedReactNode> | null | undefined, pageProps: React.JSX.IntrinsicAttributes & { children?: React.ReactNode | undefined; }) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default TreeDetailPage