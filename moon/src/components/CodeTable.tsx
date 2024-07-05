
import 'github-markdown-css/github-markdown-light.css'
import { useRouter } from 'next/router'
import Markdown from 'react-markdown'
import { formatDistance, fromUnixTime } from 'date-fns'
import folderPic from '../../public/icons/folder.svg'
import filePic from '../../public/icons/file.svg'
import Image from 'next/image'
import styles from './CodeTable.module.css'

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
        <div className= {styles.dirTable} style={fileCodeContainerStyle}>
            <div className={styles.innerTable}>
                <table className={styles.dirShowTable}>
                    <thead className={styles.dirShowTableThead}>
                        <tr>
                            <th scope="col" className={styles.dirShowTableTr}>
                                Name
                            </th>
                            <th scope="col" className={styles.dirShowTableTr}>
                                Message
                            </th>
                            <th scope="col" className={styles.dirShowTableTr}>
                                Date
                            </th>
                        </tr>
                    </thead>
                    <tbody className={styles.dirShowTableTbody}>
                        {showTree && (
                            <tr style={dirShowTrStyle} className={styles.dirShowTr} key="back">
                                <td className={styles.projectName}>
                                    <Image src={folderPic} alt="File icon" className={styles.fileTableIcon} />
                                    <span onClick={() => handleGoBack()}>..</span>
                                </td>
                                <td></td>
                                <td></td>
                            </tr>
                        )}

                        {sortedProjects.map((project) => (
                            <tr style={dirShowTrStyle} className={styles.dirShowTr} key={project.id}>
                                {project.content_type === 'file' && (
                                    <td className={styles.projectName} >
                                        <Image src={filePic} alt="File icon" className={styles.fileTableIcon} />
                                        <span onClick={() => handleFileClick(project)}>{project.name}</span>
                                    </td>
                                )}
                                {project.content_type === 'directory' && (
                                    <td className={styles.projectName} >
                                        <Image src={folderPic} alt="File icon" className={styles.fileTableIcon} />
                                        <span onClick={() => handleDirectoryClick(project)}>{project.name}</span>
                                    </td>
                                )}
                                <td className={styles.projectCommitMsg} >{project.message}</td>
                                <td className={styles.projectCommitMsg}>
                                    {project.date && formatDistance(fromUnixTime(project.date), new Date(), { addSuffix: true })}
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>
            {readmeContent && (
                <div className={styles.markdownContent}>
                    <div className="markdown-body">
                        <Markdown>{readmeContent}</Markdown>
                    </div>
                </div>
            )}
        </div>
    );
};



export default CodeTable;
