'use client'

import CodeTable from '@/components/CodeTable';
import { useEffect, useState } from 'react';

export default function HomePage() {
  const [directory, setDirectory] = useState([]);
  const [readmeContent, setReadmeContent] = useState("");

  const fetchData = async () => {
    try {
      const directory = await getDirectory("/");
      setDirectory(directory);
      const readmeContent = await getReadmeContent("/", directory);
      setReadmeContent(readmeContent);
    } catch (error) {
      console.error('Error fetching data:', error);
    }
  };

  useEffect(() => {
    fetchData();
  }, []);

  return (
    <div>
      <CodeTable directory={directory} readmeContent={readmeContent} />
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
