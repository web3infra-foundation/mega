import axios from 'axios';
import { useState, useEffect } from "react";
import Bottombar from '../components/Bottombar';
import TopNavbar from '../components/TopNavbar';
import '../styles/index.css';
import { useRouter } from 'next/router';



const HomePage = ({ directory, readmeContent, fileContent }) => {
    // 获取路由对象
    const router = useRouter();
    const currentProjectDir = directory.items || [];
    console.log(directory);
    const [hasReadme, setHasReadme] = useState(!!readmeContent);


    const [currentFileContent, setCurrentFileContent] = useState("");

    const handleFileClick = (file) => {
        router.push(`/?object_id=${file.id}`);
    };

    const handleDirectoryClick = (directory) => {
        router.push(`/?repo_path=${directory.repo_path}&object_id=${directory.id}`);
    };


    // 根据文件类型进行排序，文件夹类型先渲染
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
        <div>
            <TopNavbar />

            <div className="dirTable px-4 sm:px-6 lg:px-8">
                <div className="mt-8 flow-root">
                    <div className="-mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
                        <div className="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
                            <div className="overflow-hidden shadow ring-1 ring-black ring-opacity-5 sm:rounded-lg">
                                <table className="min-w-full divide-y divide-gray-300">
                                    <thead className="bg-gray-50">
                                        <tr>
                                            <th scope="col" className="py-3.5 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 sm:pl-6">
                                                Name
                                            </th>
                                            <th scope="col" className="px-3 py-3.5 text-left text-sm font-semibold text-gray-900">
                                                commit
                                            </th>
                                            <th scope="col" className="px-3 py-3.5 text-left text-sm font-semibold text-gray-900">
                                                commitData
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-gray-200 bg-white">
                                        {sortedProjects.map((project) => (
                                            <tr key={project.id}>
                                                {project.content_type === 'file' && (
                                                    <td className="projectName whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6">
                                                        <img src="/icons/file.svg" className='fileTableIcon' alt="File icon" />
                                                        <span onClick={() => handleFileClick(project)}>{project.name}</span>
                                                    </td>
                                                )}
                                                {project.content_type === 'directory' && (
                                                    <td className="projectName whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6">
                                                        <img src="/icons/folder.svg" className='fileTableIcon' alt="File icon" />
                                                        <span onClick={() => handleDirectoryClick(project)}>{project.name}</span>
                                                    </td>
                                                )}
                                                <td className="whitespace-nowrap px-3 py-4 text-sm text-gray-500">{project.commit_msg}</td>
                                                <td className="whitespace-nowrap px-3 py-4 text-sm text-gray-500">{project.commit_date}</td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            {hasReadme && (
                <div className="mt-8 readmeContainer">
                    <h2 className="text-2xl font-semibold mb-2">README.md</h2>
                    <div className="px-4 sm:px-6 lg:px-8">
                        <pre> {readmeContent}</pre>
                    </div>
                </div>
            )}
            <Bottombar />
        </div>
    );
};

export async function getServerSideProps(context) {
    // 设置MEGA_URL
    const MEGA_URL = 'http://localhost:8000';

    // 从上下文中获取请求参数
    const { repo_path, object_id } = context.query;

    // 获取当前目录，根目录只有path参数，没有id参数。非根目录二者皆有
    const response = repo_path && object_id
        ? await axios.get(`${MEGA_URL}/api/v1/tree?repo_path=/projects/freighter&object_id=${encodeURIComponent(object_id)}`)
        : await axios.get(`${MEGA_URL}/api/v1/tree?repo_path=/projects/freighter`);

    const directory = response.data;
    // 检测README 文件
    var readmeContent = '';
    var fileContent = '';

    if (object_id) {
        try {
            const fileResponse = await axios.get(`${MEGA_URL}/api/v1/blob?object_id=${object_id}`, { withCredentials: true });
            fileContent = fileResponse.data.row_data;
        } catch (error) {
            console.error("Error fetching README content:", error);
        }
    }

    for (const project of directory.items || []) {
        if (project.name === 'README.md' && project.content_type === 'file') {
            try {
                const response = await axios.get(`${MEGA_URL}/api/v1/blob?object_id=${project.id}`, { withCredentials: true });
                readmeContent = response.data.row_data;
                setHasReadme(true);
                break;  // 找到 README 后，不再遍历
            } catch (error) {
                console.error("Error fetching README content:", error);
            }
        }
    }

    return {
        props: {
            directory,
            readmeContent,
            fileContent,
        },
    };
}


export default HomePage;
