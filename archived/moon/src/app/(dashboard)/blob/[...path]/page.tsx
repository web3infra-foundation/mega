'use client'

import CodeContent from '@/components/CodeContent';
import Bread from '@/components/BreadCrumb';
import React, { useEffect, useState } from 'react';
import { Flex, Layout } from 'antd';

type Params = Promise<{ path: string[] }>

export default function BlobPage({ params }: { params: Params }) {
    const { path } = React.use(params);

    const new_path = '/' + path.join('/');
    const [fileContent, setFileContent] = useState("");
    useEffect(() => {
        const fetchData = async () => {
            try {
                const fileContent = await getFileContent(new_path);
                setFileContent(fileContent);
            } catch (error) {
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
