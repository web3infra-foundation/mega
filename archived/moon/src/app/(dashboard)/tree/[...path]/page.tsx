'use client'

import CodeTable from '@/components/CodeTable';
import Bread from '@/components/BreadCrumb';
import RepoTree from '@/components/RepoTree';
import CloneTabs from '@/components/CloneTabs';
import React, {useEffect, useState} from 'react';
import {Flex, Layout} from 'antd'

type Params = Promise<{ path: string[] }>

export default function Page({ params }: { params: Params }) {
    const { path } = React.use(params);

    const [directory, setDirectory] = useState([]);
    const [readmeContent, setReadmeContent] = useState("");
    const [cloneBtn, setCloneBtn] = useState(true);
    const [endpoint, setEndPoint] = useState("");
    const new_path = '/' + path.join('/');
    useEffect(() => {
        const fetchData = async () => {
            try {
                const directory = await getDirectory(new_path);
                setDirectory(directory);
                const readmeContent = await getReadmeContent(new_path, directory);
                setReadmeContent(readmeContent);
                const shown_clone_btn = await pathCanClone(new_path);
                setCloneBtn(shown_clone_btn);
                const endpoint = await getEndpoint();
                setEndPoint(endpoint);
            } catch (error) {
                console.error('Error fetching data:', error);
            }
        };
        fetchData();
    }, [new_path]);

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
                {
                    cloneBtn &&
                    <Flex justify={'flex-end'} >
                        <CloneTabs endpoint={endpoint} />
                    </Flex>
                }
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
    return response.data.data
}

async function getReadmeContent(pathname, directory) {
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

async function pathCanClone(pathname: string) {
    const res = await fetch(`/api/tree/path-can-clone?path=${pathname}`);
    const response = await res.json();
    return response.data.data
}


async function getEndpoint() {
    const res = await fetch(`/host`);
    const response = await res.json();
    return response.endpoint
}