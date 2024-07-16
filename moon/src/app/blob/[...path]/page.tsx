'use client'
import CodeContent from '@/components/CodeContent';
import Bread from '@/components/BreadCrumb';
import { useEffect, useState } from 'react';

export default function BlobPage({ params }: { params: { path: string[] } }) {
    let path = '/' + params.path.join('/');
    const [readmeContent, setReadmeContent] = useState("");
    useEffect(() => {
        const fetchData = async () => {
            try {
                let readmeContent = await getReadmeContent(path);
                setReadmeContent(readmeContent);
            } catch (error) {
                console.error('Error fetching data:', error);
            }
        };
        fetchData();
    }, [path]);

    return (
        <div>
            <Bread path={params.path} />
            <CodeContent fileContent={readmeContent} />
        </div>
    )
}

export async function getReadmeContent(pathname: string) {
    const res = await fetch(`http://localhost:3000/api/blob/?path=/${pathname}`);
    const response = await res.json();
    const directory = response.data.data;
    return directory
}
