import { useRouter } from 'next/router';
import Image from "next/legacy/image";
import './globals.css';
import './index.css';
import megaImg from '../../public/imgs/mega.png';
import CodeIconImg from '../../public/imgs/code.svg';
import IssueIconImg from '../../public/imgs/issues.svg';
import PRIconImg from '../../public/imgs/git-pull-request.svg';
import HeadImg from '../../public/logo192.png';
import HelpImg from '../../public/imgs/help.svg';
import NotificationImg from '../../public/imgs/notification.svg';
import { useState, useEffect } from 'react'
import axios from 'axios';
import ReactMarkdown from 'react-markdown';
import 'github-markdown-css/github-markdown-light.css';

const data = {
    id: 'root',
    name: 'Parent',
    children: [
        {
            id: '1',
            name: 'Child - 1',
        },
        {
            id: '2',
            name: 'Child - 2',
            children: [
                {
                    id: '3',
                    name: 'Child - 3',
                },
            ],
        },
    ],
};


const lastCommitInfo = [
    { name: 'LL', action: ' Merge pull request', actionLink: '#72', content: 'from benjamin-747/main' }
]

const commit_amount = 161;
const branch_amount = 2;
const tag_amount = 7;

function classNames(...classes) {
    return classes.filter(Boolean).join(' ')
}

const default_md_content = "## Welcome to mega!##";

export default function HomePage() {
    const [dir_file_data, setdir_file_data] = useState(null);
    const [readmeContent, setReadmeContent] = useState(null);


    useEffect(() => {
        async function get_file_dir() {
            try {
                const response = await axios.get('/api/v1/tree?repo_path=/root/mega');
                setdir_file_data(response.data);
                console.log(dir_file_data);
                // 检查是否有 README.md 文件
                const readmeFile = response.data.items.find(item => item.name === 'README.md');
                if (readmeFile && readmeFile.content_type === 'file') {
                    async function getReadmeContent() {
                        try {
                            const response = await axios.get(`/api/v1/blob?repo_path=/root/mega&object_id=${readmeFile.id}`);
                            setReadmeContent(response.data.row_data);
                        } catch (error) {
                            console.error(error);
                        }
                    }
                    getReadmeContent();
                } else {
                    setReadmeContent(default_md_content);
                }
            } catch (error) {
                console.error(error);
            }
        }
        get_file_dir();
    }, []);
    const router = useRouter();

    const handleItemClick = (item) => {
        router.push({
            pathname: '/codeViewComponent',
            query: { itemId: item.id, itemType: item.content_type }
        });
    };
    const get_icon_for_content_type = (content_type) => {
        if (content_type === 'directory') {
            return (
                <svg t="1690163488952" className="icon w-5 h-5 text-gray-400" viewBox="0 0 1024 1024">
                    <path d="M81.16 412.073333L0 709.653333V138.666667a53.393333 53.393333 0 0 1 53.333333-53.333334h253.413334a52.986667 52.986667 0 0 1 37.713333 15.62l109.253333 109.253334a10.573333 10.573333 0 0 0 7.54 3.126666H842.666667a53.393333 53.393333 0 0 1 53.333333 53.333334v74.666666H173.773333a96.2 96.2 0 0 0-92.613333 70.74z m922-7.113333a52.933333 52.933333 0 0 0-42.386667-20.96H173.773333a53.453333 53.453333 0 0 0-51.453333 39.333333L11.773333 828.666667a53.333333 53.333333 0 0 0 51.453334 67.333333h787a53.453333 53.453333 0 0 0 51.453333-39.333333l110.546667-405.333334a52.953333 52.953333 0 0 0-9.073334-46.373333z" fill="#515151" p-id="16616"></path>
                </svg>
            );
        } else if (content_type === 'file') {
            return (
                <svg t="1690163488952" className="icon w-6 h-6 text-gray-400" viewBox="0 0 1024 1024">
                    <path d="M935.082667 480a401.194667 401.194667 0 0 0-1.194667-10.666667h-0.170667c0.426667 3.541333 1.109333 7.04 1.365334 10.666667zM931.285333 450.474667l0 0zM933.717333 469.333333h0.170667a502.613333 502.613333 0 0 0-2.56-18.858666c0.853333 6.272 1.578667 12.586667 2.389333 18.858666zM810.666667 297.088c0-11.093333-10.453333-19.669333-16.042667-22.016l-136.832-131.882667C655.402667 137.813333 646.016 128 634.453333 128H597.333333v213.333333h213.333334V297.088z" fill="#707070" p-id="3107"></path><path d="M554.666667 128H261.12C233.685333 128 213.333333 150.826667 213.333333 177.28v670.805333C213.333333 874.538667 233.642667 896 261.12 896h497.792c27.477333 0 51.754667-21.461333 51.754667-47.914667V384h-256V128z m149.546666 640H318.72C307.626667 768 298.666667 757.674667 298.666667 746.666667s8.96-21.333333 20.010666-21.333334h385.536c11.008 0 20.010667 10.325333 20.010667 21.333334s-9.002667 21.333333-20.010667 21.333333z m0-213.333333c11.008 0 20.010667 10.325333 20.010667 21.333333s-9.002667 21.333333-20.010667 21.333333H318.72C307.626667 597.333333 298.666667 587.008 298.666667 576s8.96-21.333333 20.010666-21.333333h385.536z" fill="#707070" p-id="3108"></path>                </svg>
            );
        }
        return null;
    };

    return (
        <>
            <div className="h-100%  w-100vm ">
                {/* 居中显示 */}
                <div className="component_box box-content h-fit p-center  ">
                    {/* 最上层的顶部固定模块 */}
                    <div className="nav_module flex bg-black-tran shadow-md">
                        {/* 头部组件中的第一层 */}
                        <div className='flex repo_info_module'>
                            <a onClick={() => router.push("/")} >
                                <Image
                                    className="rounded-md mega_image"
                                    src={megaImg}
                                    alt="megaImg"
                                    width={35}
                                    height={35}
                                >
                                </Image>
                            </a>
                            <div className='float-left h-35 line-height-35 text-black pl-4'>
                                MEGA
                            </div>
                        </div>
                        <div className='repo_funtion_link p-center flex'>
                            <div className="module_link">
                                <div className="module_link_icon h-35 line-height-35 pt-1">
                                    <Image
                                        className="rounded-full mega_image"
                                        src={CodeIconImg}
                                        alt="CodeIconImg"
                                        width={20}
                                        height={20}
                                    />
                                </div>
                                <button className="CodeTitle h-35 line-height-35 " onClick={() => router.push('/codeViewComponent')}>&ensp;Code</button>
                            </div>
                            <div className="module_link">
                                <div className="module_link_icon h-35 line-height-35 pt-1">
                                    <Image
                                        className="rounded-full mega_image"
                                        src={IssueIconImg}
                                        alt="CodeIconImg"
                                        width={20}
                                        height={20}
                                    />
                                </div>
                                <button className="CodeTitle line-height-35">&ensp;Issues</button>
                            </div>
                            <div className="module_link">
                                <div className="module_link_icon h-35 line-height-35 pt-1">
                                    <Image
                                        className="rounded-full mega_image"
                                        src={PRIconImg}
                                        alt="CodeIconImg"
                                        width={20}
                                        height={20}
                                    />
                                </div>

                                <button className="CodeTitle h-35 line-height-35">&ensp;Merge requests</button>
                            </div>
                        </div>

                        <div className='flex self_info_module'>
                            <div className='search_module flex'>
                                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5 text-gray-400 search_icon_input">
                                    <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
                                </svg>
                                <input className="search_pr_input h-9 rounded-md"></input>
                            </div>
                            <a className="self_image self_image_others mt-1" href='/'>
                                <Image
                                    className="rounded-full"
                                    src={NotificationImg}
                                    alt="NotificationImg"
                                    width={25}
                                    height={25}
                                >
                                </Image>
                            </a>
                            <a className="self_image self_image_others mt-1" href='/'>
                                <Image
                                    className="rounded-full"
                                    src={HelpImg}
                                    alt="HelpImg"
                                    width={25}
                                    height={25}
                                >
                                </Image>
                            </a>
                            <a className="self_image HeadImg" href='/'>
                                <Image
                                    className="rounded-full"
                                    src={HeadImg}
                                    alt="HeadImg"
                                    width={35}
                                    height={35}
                                >
                                </Image>
                            </a>
                        </div>
                    </div>
                    {/* 下层的主页展示模块 */}
                    <div className=" w-index p-center">
                        {/* 展示文件目录 */}
                        <div className="front-show-body">
                            <div className="mt-4 flow-root">
                                <div className="-mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
                                    <div className="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
                                        <div className="overflow-hidden shadow ring-1 ring-black ring-opacity-5 sm:rounded-lg">
                                            <table className="min-w-full divide-y divide-gray-300">
                                                <thead className="bg-gray-100">
                                                    <tr>
                                                        {/* 显示最新提交记录 */}
                                                        <th scope="col" className="pt-1 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 sm:pl-6">
                                                            <div className='absolute pb-4'>
                                                            </div>
                                                            <span className=""> {lastCommitInfo[0].name}  {lastCommitInfo[0].action}  <a href='/' className='text-blue-600'>{lastCommitInfo[0].actionLink} </a>{lastCommitInfo[0].content}</span>
                                                        </th>
                                                        <th scope="col" className="py-3.5 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 sm:pl-6">
                                                        </th>
                                                        <th scope="col" className="flex py-3.5 pl-10 pr-3 text-left text-sm font-medium text-gray-500 sm:pl-6">
                                                            <svg t="1690163488952" className="icon w-4 h-4 mt-2px text-gray-400" viewBox="0 0 1024 1024" ><path d="M108.48 909.441V114.044c0-20.114 16.065-36.42 35.882-36.42h665.107c19.817 0 35.883 16.306 35.883 36.42v399.612h55.773V61.143c0.001-22.817-18.496-41.314-41.313-41.314H94.019c-22.817 0-41.314 18.497-41.314 41.314V963.42c0 22.817 18.497 41.314 41.314 41.314h384.968v-58.872H144.362c-19.817 0-35.882-16.306-35.882-36.421z" fill="#999999" p-id="6011"></path><path d="M224.597 334.281h504.637c14.912 0 27-12.088 27-27s-12.088-27-27-27H224.597c-14.912 0-27 12.088-27 27s12.088 27 27 27zM224.597 515.281h504.637c14.912 0 27-12.088 27-27s-12.088-27-27-27H224.597c-14.912 0-27 12.088-27 27s12.088 27 27 27zM226.296 696.281h504.637c14.912 0 27-12.088 27-27s-12.088-27-27-27H226.296c-14.912 0-27 12.088-27 27s12.088 27 27 27z" fill="#999999" p-id="6012"></path><path d="M964.387 601.815c-10.544-10.544-27.64-10.544-38.184 0L588.775 939.243 385.284 735.752c-10.544-10.544-27.64-10.544-38.184 0-10.544 10.544-10.544 27.64 0 38.184l219.161 219.161 2 2 4.844 4.844c10.524 7.297 25.075 6.266 34.45-3.11l19.404-19.404 337.428-337.428c10.544-10.545 10.544-27.64 0-38.184z" fill="#999999" p-id="6013"></path></svg>
                                                            <a href='/' className='hover:text-blue-600'>&thinsp;{commit_amount} commit</a>
                                                        </th>
                                                    </tr>
                                                </thead>
                                                <tbody className="divide-y divide-gray-200 bg-white">
                                                    {dir_file_data && dir_file_data.items.map((item) => (
                                                        <tr key={item.name}>
                                                            <td className="flex whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6">
                                                                {get_icon_for_content_type(item.content_type)}
                                                                &ensp;
                                                                <a className='file_dir_link' onClick={() => handleItemClick(item)}>{item.name}</a>
                                                            </td>
                                                            <td className="whitespace-nowrap px-3 py-4 text-sm text-gray-500 text_over_right">{item.mr_msg}</td>
                                                            <td className="whitespace-nowrap pl-7 py-4 text-sm text-gray-500 text_over_left">{item.mr_date}</td>
                                                        </tr>
                                                    ))}
                                                </tbody>
                                            </table>
                                        </div>
                                        <div className='the_markdown_content'>
                                            <div className='markdown_content_nav'>
                                                <a onClick={() => router.push("/codeViewComponent")}>README.md</a>
                                            </div>
                                            <div className="markdown-body">
                                                <ReactMarkdown>{readmeContent}</ReactMarkdown>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div >
        </>
    );
}

