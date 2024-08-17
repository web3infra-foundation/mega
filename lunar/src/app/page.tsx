'use client'

import { Flex, Layout, Skeleton, Alert } from "antd/lib";
import CodeTable from '@/components/CodeTable';
import MergeList from '@/components/MergeList';
import { useTreeCommitInfo, useBlobContent, useMRList } from '@/app/api/fetcher';

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

  if (isTreeLoading || isBlobLoading || isMRLoading) return <Skeleton />;

  return (
    <div>
      <Alert
        banner
        message={
          "Relay address is not configed, Some functions are not available"
        }
      />
      {
        <Flex gap="middle" wrap>
          <Layout style={leftStyle}>
            {
              (tree && blob) &&
              <CodeTable directory={tree.data} readmeContent={blob.data} />
            }
          </Layout>
          <Layout style={rightStyle}>
            {(isTreeLoading || isBlobLoading) &&
              <Skeleton />
            }
            {(!isTreeLoading && !isBlobLoading) &&
              <MergeList mrList={mrList.data} />
            }
            {/* <Content style={contentStyle}></Content> */}
          </Layout>
        </Flex>
      }
    </div>
  )
}
