'use client'

import CodeTable from './CodeTabe';
import { useEffect, useState } from 'react';

export default function TestView() {
  const [directory, setDirectory] = useState([]);
  const [readmeContent, setReadmeContent] = useState("");

  const fetchData = async () => {
    try {
      const directory = await getDirectory("/");

      setDirectory(directory);
      const readmeContent = await getReadmeContent("/", directory);

      setReadmeContent(readmeContent);
    } catch (error) {
      // console.error('Error fetching data:', error);
    }
  };

  useEffect(() => {
    fetchData();
  }, []);

  return (
    <div className='p-3.5 mt-3'>
      <CodeTable directory={directory} readmeContent={readmeContent} />
    </div>
  );
}
async function getDirectory(pathname: string) {
  const res = await fetch(`/api/tree/commit-info?path=${pathname}`);
  const response = await res.json();
  const directory = response?.data?.data;

  return directory
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
