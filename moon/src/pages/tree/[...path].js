import { useRouter } from 'next/router';
import CodeTable from '@/components/CodeTable'
import axios from 'axios';

export default function TreePage({ directory, readmeContent, fileContent }) {

    return (
        <div>
            <CodeTable directory={directory} readmeContent={readmeContent} fileContent={fileContent} />
        </div>
    );
}

export async function getServerSideProps(context) {
    const MEGA_URL = 'http://localhost:8000';
    // get the parameters form context
    const { path } = context.query;
    
    // obtain the current directory
    const response = await axios.get(`${MEGA_URL}/api/v1/tree-commit-info?path=/${encodeURIComponent(path.join('/'))}`);
    const directory = response.data;
    var readmeContent = '';

    // get the readme file content
    for (const project of directory.data || []) {
        if (project.name === 'README.md' && project.content_type === 'file') {
            try {
                const response = await axios.get(`${MEGA_URL}/api/v1/blob?path=/${path.join('/')}/README.md`, { withCredentials: true });
                readmeContent = response.data.data;
                break;
            } catch (error) {
                console.error("Error fetching README content:", error);
            }

        }
    }


    return {
        props: {
            directory,
            readmeContent,
        },
    };
}
