'use client'

import CodeTable from './CodeTable';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useGetTreeCommitInfo } from '@/hooks/useGetTreeCommitInfo';
import { CommonResultVecTreeCommitItem } from '@gitmono/types/generated';

export default function TestView() {
  const { data:TreeCommitInfo } = useGetTreeCommitInfo('/')

  type DirectoryType = NonNullable<CommonResultVecTreeCommitItem['data']>;
  const directory:DirectoryType = useMemo(() => TreeCommitInfo?.data ?? [], [TreeCommitInfo]);

  const [readmeContent, setReadmeContent] = useState("");

  const fetchData = useCallback(async () => {
    if (directory.length === 0) return; 

    try {
      const content = await getReadmeContent("/", directory);

      setReadmeContent(content);
    } catch (error) {
      // console.error('Error fetching data:', error);
    }
  }, [directory]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);
  
  return (
    <div className='p-3.5 mt-3'>
      <CodeTable directory={directory} readmeContent={readmeContent} />
    </div>
  );
}

async function getReadmeContent(pathname:string, directory: any) {
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
