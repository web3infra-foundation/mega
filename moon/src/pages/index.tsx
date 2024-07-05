import CodeTable from '../components/CodeTable';

export default function HomePage({ directory, readmeContent }) {
    return (
        <div>
            <CodeTable directory={directory} readmeContent={readmeContent} showTree={false} />
        </div>
    );
}

export async function getServerSideProps(context) {
    const { path } = context.query;
    // obtain the current directory
    const res = await fetch(`http://localhost:3000/api/tree-commit?path=/`);
    const response = await res.json();
    const directory = response.data;
    var readmeContent = '';
    // get the readme file content
    for (const project of directory || []) {
        if (project.name === 'README.md' && project.content_type === 'file') {
            const res = await fetch(`http://localhost:3000/api/blob?path=/${path.join('/')}/README.md`);
            const response = await res.json();
            readmeContent = response.data;
            break;
        }
    }

    return {
        props: {
            directory,
            readmeContent,
        },
    };
}
