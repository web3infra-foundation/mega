'use client'

import { Flex, Layout } from 'antd';
import { Skeleton } from "antd";
import CodeTable from '../../../moon/src/components/CodeTable';
import MergeList from '../../../moon/src/components/MergeList';
import { useTreeCommitInfo, useBlobContent } from '@/app/api/fetcher';

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

  return (
    <Flex gap="middle" wrap>
      <Layout style={layoutStyle}>
        {(isTreeLoading || isBlobLoading) &&
          <Skeleton />
        }
        {
          (tree && blob) &&
          <CodeTable directory={tree.data} readmeContent={blob.data} treeIsShow={false} />
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
  )
}
