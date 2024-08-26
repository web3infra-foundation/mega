'use client'

import CodeTable from '@/components/CodeTable'
import Bread from '@/components/BreadCrumb'
import RepoTree from '@/components/RepoTree'
import { useBlobContent, useMegaStatus, useTreeCommitInfo } from '@/app/api/fetcher'
import { useSearchParams } from 'next/navigation';
import { Skeleton, Flex, Layout } from "antd/lib";
import { Suspense } from 'react'

export default function Page() {
    return (
        <Suspense>
            <Tree />
        </Suspense>
    );
}

function Tree() {
    const searchParams = useSearchParams();
    const path = searchParams.get('path');
    const { tree, isTreeLoading, isTreeError } = useTreeCommitInfo(path);
    const { blob, isBlobLoading, isBlobError } = useBlobContent(`${path}/README.md`);
    const { status, isLoading, isError } = useMegaStatus();

    if (isTreeLoading || isBlobLoading) return <Skeleton />;

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
        <Flex gap="middle" wrap>
            <Layout style={breadStyle}>
                <Bread path={path} />
            </Layout>
            <Layout style={treeStyle}>
                <RepoTree directory={tree.data} />
            </Layout>
            <Layout style={codeStyle}>
                <CodeTable directory={tree.data} readmeContent={blob.data} with_ztm={status[1]} />
            </Layout>
        </Flex>
    )
}