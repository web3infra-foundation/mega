'use client'

import CodeContent from '@/components/CodeView/blobView/CodeContent';
import Bread from '@/components/CodeView/TreeView/BreadCrumb';
import React, { useEffect, useState } from 'react';
import { Flex, Layout } from 'antd';
import { useParams } from 'next/navigation';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import { AppLayout } from '@/components/Layout/AppLayout';

function BlobPage() {
    const params = useParams();
    const path = Array.isArray(params?.path) ? params.path : [];
    const new_path = '/' + path.join('/');

    const [fileContent, setFileContent] = useState("");

    useEffect(() => {
        const fetchData = async () => {
            try {
                const fileContent = await getFileContent(new_path);

                setFileContent(fileContent);
            } catch (error) {
                // eslint-disable-next-line no-console
                console.error('Error fetching data:', error);
            }
        };

        fetchData();
    }, [new_path]);

    const treeStyle = {
        borderRadius: 8,
        overflow: 'hidden',
        width: 'calc(15% - 8px)',
        maxWidth: 'calc(15% - 8px)',
        background: '#fff',
    };

    const codeStyle = {
        borderRadius: 8,
        overflow: 'hidden',
        width: 'calc(85% - 8px)',
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
        <div>
            <Flex gap="middle" wrap>
                <Layout style={breadStyle}>
                    <Bread path={path} />
                </Layout>
                <Layout style={treeStyle}>
                    {/* <RepoTree directory={directory} /> */}
                </Layout>
                <Layout style={codeStyle}>
                    <CodeContent fileContent={fileContent} />
                </Layout>
            </Flex>
        </div>
    )
}

async function getFileContent(pathname: string) {
    const res = await fetch(`/api/blob?path=${pathname}`);
    const response = await res.json();
    const directory = response.data.data;

    return directory
}


BlobPage.getProviders = (page: string | number | boolean | React.ReactElement<any, string | React.JSXElementConstructor<any>> | Iterable<React.ReactNode> | React.ReactPortal | Promise<React.AwaitedReactNode> | null | undefined, pageProps: React.JSX.IntrinsicAttributes & { children?: React.ReactNode | undefined; }) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default BlobPage