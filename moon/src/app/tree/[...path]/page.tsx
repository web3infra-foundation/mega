'use client'

import CodeTable from '@/components/CodeTable'
import Bread from '@/components/BreadCrumb'
import RepoTree from '@/components/RepoTree'
import { useEffect, useState } from 'react'
import { Flex, Layout } from "antd/lib";

export default function Page({ params }: { params: { path: string[] } }) {
    const [directory, setDirectory] = useState([]);
    const [readmeContent, setReadmeContent] = useState("");
    let path = '/' + params.path.join('/');
    useEffect(() => {
        const fetchData = async () => {
            try {
                let directory = await getDirectory(path);
                setDirectory(directory);
                let readmeContent = await getReadmeContent(path, directory);
                setReadmeContent(readmeContent);
            } catch (error) {
                console.error('Error fetching data:', error);
            }
        };
        fetchData();
    }, [path]);

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
                <Bread path={params.path} />
            </Layout>
            <Layout style={treeStyle}>
                <RepoTree directory={directory} />
            </Layout>
            <Layout style={codeStyle}>
                <CodeTable directory={directory} readmeContent={readmeContent} />
            </Layout>
        </Flex>
    );
}

async function getDirectory(pathname: string) {
    const res = await fetch(`/api/tree/commit-info?path=${pathname}`);
    const response = await res.json();
    const directory = response.data.data;
    return directory
}

async function getReadmeContent(pathname, directory) {
    var readmeContent = '';
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
