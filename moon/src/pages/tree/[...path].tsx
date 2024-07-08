import CodeTable from '../../components/CodeTable'
import Bread from '../../components/BreadCrumb'
import RepoTree from '../../components/RepoTree'

export default function TreePage({ directory, readmeContent }) {
    return (
        <div>
            <RepoTree directory={directory} />
            <Bread />
            <CodeTable directory={directory} readmeContent={readmeContent} showTree={true} />
        </div>
    );
}

export async function getServerSideProps(context) {
    const { path } = context.query;
    // obtain the current directory
    const res = await fetch(`http://localhost:3000/api/tree-commit?path=/${path.join('/')}`);
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
