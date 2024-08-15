'use client'

import { Flex, Layout } from 'antd';
import { Skeleton, Button, Result } from "antd/lib";
import CodeTable from '@/components/CodeTable';
import MergeList from '@/components/MergeList';
import { useTreeCommitInfo, useBlobContent, useMegaStatus } from '@/app/api/fetcher';

const { Content } = Layout;

const contentStyle: React.CSSProperties = {
  textAlign: 'center',
  minHeight: 500,
  lineHeight: '120px',
  border: 'solid',
  backgroundColor: '#fff',
};

const layoutStyle = {
  minHeight: 500,
  borderRadius: 8,
  overflow: 'hidden',
  width: 'calc(50% - 8px)',
  maxWidth: 'calc(50% - 8px)',
  background: '#fff'
};

const mrList = [
  {
    "id": 2278111790530821,
    "title": "",
    "status": "open",
    "open_timestamp": 1721181311,
    "merge_timestamp": null
  },
  {
    "id": 2277296526688517,
    "title": "",
    "status": "merged",
    "open_timestamp": 1721131551,
    "merge_timestamp": 1721131565
  },
  {
    "id": 2276683620876549,
    "title": "",
    "status": "merged",
    "open_timestamp": 1721094142,
    "merge_timestamp": 1721117874
  }
];

export default function HomePage() {
  const { tree, isTreeLoading, isTreeError } = useTreeCommitInfo("/");
  const { blob, isBlobLoading, isBlobError } = useBlobContent("/README.md");
  const { status, isLoading, isError } = useMegaStatus();

  if (isTreeLoading || isBlobLoading || isLoading) return <Skeleton />;

  return (
    <div>
      {!status &&
        < Result
          status="warning"
          title="Set relay server address first to start mega server"
          extra={
            <Button type="primary" key="setting" href='/settings'>
              Go Setting
            </Button>
          }
        />
      }
      {
        status &&
        <Flex gap="middle" wrap>
          <Layout style={layoutStyle}>
            {
              (tree && blob) &&
              <CodeTable directory={tree.data} readmeContent={blob.data} />
            }
          </Layout>
          <Layout style={layoutStyle}>
            {(isTreeLoading || isBlobLoading) &&
              <Skeleton />
            }
            {(!isTreeLoading && !isBlobLoading) &&
              <MergeList mrList={mrList} />
            }
            {/* <Content style={contentStyle}></Content> */}
          </Layout>
        </Flex>
      }
    </div>
  )
}
