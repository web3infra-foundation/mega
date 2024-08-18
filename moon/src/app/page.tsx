import CodeTable from '@/components/CodeTable'
export const revalidate = 0

export default async function HomePage() {
  let directory = await getDirectory()
  let readmeContent = await getReadmeContent(directory)

  return (
    <div>
      <CodeTable directory={directory} readmeContent={readmeContent} treeIsShow={false} />
    </div>
  );
}

async function getDirectory() {
  const res = await fetch(`http://localhost:3000/api/tree/commit-info?path=/`);

  const response = await res.json();
  const directory = response.data.data;

  return directory
}

async function getReadmeContent(directory) {
  let readmeContent = '';

  for (const project of directory || []) {
    if (project.name === 'README.md' && project.content_type === 'file') {
      const res = await fetch(`http://localhost:3000/api/blob?path=/README.md`);
      const response = await res.json();
      readmeContent = response.data.data;
      break;
    }
  }

  return readmeContent
}
