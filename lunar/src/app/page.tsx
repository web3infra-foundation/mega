'use client'

import { Flex, Layout, Skeleton } from "antd/lib";
import CodeTable from '@/components/CodeTable';
import MergeList from '@/components/MergeList';
import { useTreeCommitInfo, useBlobContent, useMRList, useMegaStatus } from '@/app/api/fetcher';

const { Content } = Layout;

const contentStyle: React.CSSProperties = {
  textAlign: 'center',
  minHeight: 500,
  lineHeight: '120px',
  border: 'solid',
  backgroundColor: '#fff',
};

const rightStyle = {
  minHeight: 768,
  borderRadius: 8,
  overflow: 'hidden',
  width: 'calc(40% - 8px)',
  maxWidth: 'calc(40% - 8px)',
  background: '#fff'
};


const leftStyle = {
  minHeight: '100%',
  borderRadius: 8,
  overflow: 'hidden',
  width: 'calc(60% - 8px)',
  maxWidth: 'calc(70% - 8px)',
  background: '#fff'
};


export default function HomePage() {
  const { tree, isTreeLoading, isTreeError } = useTreeCommitInfo("/");
  const { blob, isBlobLoading, isBlobError } = useBlobContent("/README.md");
  const { mrList, isMRLoading, isMRError } = useMRList("");
  const { status, isLoading, isError } = useMegaStatus();

  if (isTreeLoading || isBlobLoading || isMRLoading || isLoading) return <Skeleton />;

  return (
    <Flex gap="middle" wrap>
      <Layout style={leftStyle}>
        {
          (tree && blob) &&
          <CodeTable directory={tree.data} readmeContent={blob.data} with_ztm={status[1]} />
        }
      </Layout>
      <Layout style={rightStyle}>
        {mrList &&
          <MergeList mrList={mrList.data} />
        }
        {/* <Content style={contentStyle}></Content> */}
      </Layout>
    </Flex>
  )
}
