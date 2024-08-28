'use client'

import CodeTable from '@/components/CodeTable'
import Bread from '@/components/BreadCrumb'
import RepoTree from '@/components/RepoTree'
import { useEffect, useState } from 'react'

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

    return (
        <div>
            <RepoTree directory={directory} />
            <Bread path={params.path} />
            <CodeTable directory={directory} readmeContent={readmeContent} treeIsShow={true} />
        </div>
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
