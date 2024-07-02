
import 'github-markdown-css/github-markdown-light.css';
import { useRouter } from 'next/router';
import Markdown from 'react-markdown';
import { formatDistance, fromUnixTime } from 'date-fns'

const CodeTable = ({ directory, readmeContent, showTree }) => {

    const router = useRouter();
    const currentProjectDir = directory || [];
    const fileCodeContainerStyle = showTree ? { width: '80%', marginLeft: '17%', borderRadius: '0.5rem', marginTop: '10px' } : { width: '90%', margin: '0 auto', borderRadius: '0.5rem', marginTop: '10px' };
    const dirShowTrStyle = { borderBottom: '1px solid  rgba(0, 0, 0, 0.1)', }

    const handleFileClick = (file) => {
        const { path } = router.query;
        const safePath = Array.isArray(path) ? path : [];

        const newPath = `/blob/${safePath.join("/")}/${file.name}`;
        router.push(newPath);
    };

    const handleDirectoryClick = async (directory) => {
        const { path } = router.query;
        var newPath = '';
        if (Array.isArray(path)) {
            newPath = `/tree/${path.join("/")}/${directory.name}`;
        } else {
            newPath = `/tree/${directory.name}`;
        }
        router.push({
            pathname: newPath,
        });
    };

    const handleGoBack = () => {
        // const path = router.query.path;
        const { path } = router.query;
        const safePath = Array.isArray(path) ? path : [];

        if (safePath.length == 1) {
            router.push('/')
        } else {
            router.push(`/tree/${safePath.slice(0, -1).join('/')}`);

        }
    };

    // sort by file type, render folder type first
    const sortedProjects = currentProjectDir.sort((a, b) => {
        if (a.content_type === 'directory' && b.content_type === 'file') {
            return -1;
        } else if (a.content_type === 'file' && b.content_type === 'directory') {
            return 1;
        } else {
            return 0;
        }
    });

    return (
        <div className="dirTable" style={fileCodeContainerStyle}>
            <div className="innerTable">
                <table className="dirShowTable">
                    <thead className="dirShowTableThead">
                        <tr>
                            <th scope="col" className="dirShowTableTr">
                                Name
                            </th>
                            <th scope="col" className="dirShowTableTr">
                                Message
                            </th>
                            <th scope="col" className="dirShowTableTr">
                                Date
                            </th>
                        </tr>
                    </thead>
                    <tbody className="dirShowTableTbody">
                        {showTree && (
                            <tr style={dirShowTrStyle} className="dirShowTr" key="back">
                                <td className="projectName ">
                                    <img src="/icons/folder.svg" className='fileTableIcon' alt="File icon" />
                                    <span onClick={() => handleGoBack()}>..</span>
                                </td>
                                <td></td>
                                <td></td>
                            </tr>
                        )}

                        {sortedProjects.map((project) => (
                            <tr style={dirShowTrStyle} className="dirShowTr" key={project.id}>
                                {project.content_type === 'file' && (
                                    <td className="projectName ">
                                        <img src="/icons/file.svg" className='fileTableIcon' alt="File icon" />
                                        <span onClick={() => handleFileClick(project)}>{project.name}</span>
                                    </td>
                                )}
                                {project.content_type === 'directory' && (
                                    <td className="projectName ">
                                        <img src="/icons/folder.svg" className='fileTableIcon' alt="File icon" />
                                        <span onClick={() => handleDirectoryClick(project)}>{project.name}</span>
                                    </td>
                                )}
                                <td className="projectCommitMsg ">{project.message}</td>
                                <td className="projectCommitMsg">
                                    {formatDistance(fromUnixTime(project.date), new Date(), { addSuffix: true })}
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>
            {readmeContent && (
                <div className='markdownContent'>
                    <div className="markdown-body">
                        <Markdown>{readmeContent}</Markdown>
                    </div>
                </div>
            )}
        </div>
    );
};



export default CodeTable;
