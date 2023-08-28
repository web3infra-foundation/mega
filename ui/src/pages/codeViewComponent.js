import './codeViewComponent.modules.css';
import './globals.css';
import TreeView from '@mui/lab/TreeView';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import TreeItem from '@mui/lab/TreeItem';
import { CheckIcon, HandThumbUpIcon, UserIcon } from '@heroicons/react/20/solid';
import 'codemirror/lib/codemirror.js';
import 'codemirror/lib/codemirror.css';
import React, { useState, useEffect } from "react";
import Editor from "../Editor.js";
import 'codemirror/mode/clike/clike';
import 'codemirror/mode/javascript/javascript';
import 'codemirror/addon/selection/active-line';
import 'codemirror/addon/fold/foldgutter.css';
import 'codemirror/addon/fold/foldcode.js';
import 'codemirror/addon/fold/foldgutter.js';
import 'codemirror/addon/fold/brace-fold.js';
import 'codemirror/addon/fold/xml-fold.js';
import 'codemirror/addon/fold/indent-fold.js';
import 'codemirror/addon/fold/markdown-fold.js';
import 'codemirror/addon/fold/comment-fold.js';
import 'codemirror/addon/scroll/simplescrollbars.js';
import 'codemirror/mode/sql/sql.js';
import 'codemirror/mode/rust/rust.js';
import ReactMarkdown from 'react-markdown';
import 'github-markdown-css/github-markdown-light.css';
import megaImg from '../../public/imgs/mega.png';
import CodeIconImg from '../../public/imgs/code.svg';
import IssueIconImg from '../../public/imgs/issues.svg';
import PRIconImg from '../../public/imgs/git-pull-request.svg';
import HeadImg from '../../public/logo192.png';
import HelpImg from '../../public/imgs/help.svg';
import NotificationImg from '../../public/imgs/notification.svg';
import Image from "next/legacy/image";
import { useRouter } from 'next/router';
import { UnControlled as CodeMirror } from 'react-codemirror2';
import Breadcrumbs from '@mui/material/Breadcrumbs';
import Link from '@mui/material/Link';
import axios from 'axios';
import { Typography } from '@mui/material';
import { useSearchParams } from 'next/navigation';

const timeline = [
  {
    id: 1,
    content: 'Applied to',
    target: 'Front End Developer',
    href: '#',
    date: 'Sep 20',
    datetime: '2020-09-20',
    icon: UserIcon,
    iconBackground: 'bg-gray-400',
  },
  {
    id: 2,
    content: 'Advanced to phone screening by',
    target: 'Bethany Blake',
    href: '#',
    date: 'Sep 22',
    datetime: '2020-09-22',
    icon: HandThumbUpIcon,
    iconBackground: 'bg-blue-500',
  },
  {
    id: 3,
    content: 'Completed phone screening with',
    target: 'Martha Gardner',
    href: '#',
    date: 'Sep 28',
    datetime: '2020-09-28',
    icon: CheckIcon,
    iconBackground: 'bg-green-500',
  },
  {
    id: 4,
    content: 'Advanced to interview by',
    target: 'Bethany Blake',
    href: '#',
    date: 'Sep 30',
    datetime: '2020-09-30',
    icon: HandThumbUpIcon,
    iconBackground: 'bg-blue-500',
  },
  {
    id: 5,
    content: 'Completed interview with',
    target: 'Katherine Snyder',
    href: '#',
    date: 'Oct 4',
    datetime: '2020-10-04',
    icon: CheckIcon,
    iconBackground: 'bg-green-500',
  },
];

const lastCommitInfo = [
  { name: 'LL', action: ' Merge pull request', actionLink: '#72', content: 'from benjamin-747/main' }
]

function classNames(...classes) {
  return classes.filter(Boolean).join(' ')
}




const getFileLanguageMode = (fileName) => {
  const fileExtension = fileName.split('.').pop();
  switch (fileExtension) {
    case 'js':
      return 'javascript';
    case 'ts':
      return 'javascript';
    case 'json':
      return 'javascript';
    case 'java':
      return 'text/x-java';
    case 'sql':
      return 'text/x-sql';
    case 'rs':
      mode = 'text/x-rust';
      break;
    // 可以继续添加其他文件类型的判断
    default:
      return null; // 返回null表示未知文件类型
  }
};

export default function Code_view() {
  const router = useRouter();
  const { itemId, itemType } = router.query;

  const [is_node_clicked, setis_node_clicked] = useState(true);
  const [is_file_clicked, setis_file_clicked] = useState(false);
  const [show_buttons, setshow_buttons] = useState(false);
  useEffect(() => {
    //使用window判断是否为在浏览器端才执行的代码
    if (typeof window !== "undefined" && is_file_clicked) {
      var Review_Canel_Button = document.getElementsByClassName("review_cancel_button");
      var Review_Post_Button = document.getElementsByClassName("review_post_button");
      var lexical_input = document.getElementsByClassName("review-Editor");
      Review_Post_Button[0].addEventListener('click', function () {
        lexical_input[0].style.display = "none";
      })
      Review_Canel_Button[0].addEventListener('click', function () {
        lexical_input[0].style.display = "none";
      })
      setshow_buttons(true);
      const handleButtonClick = (e) => {
        const lexicalInput = document.getElementsByClassName("review-Editor")[0];
        lexicalInput.style.display = "flex";
      };
      const codeMirrorInstances = document.getElementsByClassName("CodeMirror");
      const review_code = document.getElementsByClassName("CodeMirror-code");
      codeMirrorInstances[0].style.height = "fit-content";
      review_code[0].style.fontSize = "14px";
      if (codeMirrorInstances.length > 0) {
        const codeMirror = codeMirrorInstances[0].CodeMirror;
        review_code[0].style.fontFamily = "Sans-serif";
        codeMirror.setOption("gutters", [...codeMirror.getOption("gutters"), "code-line-buttons"]);
        for (let i = 0; i < codeMirror.lineCount(); i++) {
          const button = document.createElement("button");
          button.textContent = "+";
          button.className = "review_button";
          button.addEventListener("click", handleButtonClick);
          codeMirror.setGutterMarker(i, "code-line-buttons", button);
        }
      }
    }
  });

  useEffect(() => {
    if (is_file_clicked) {
      var code_review_timeline = document.getElementsByClassName("code-review-timeline");
      var show_code = document.getElementsByClassName("CodeMirror");
      var review_model_button = document.getElementsByClassName("review-model-button");
      var code_model_button = document.getElementsByClassName("code-model-button");
      review_model_button[0].addEventListener('click', function () {
        show_code[0].style.display = "none";
        code_review_timeline[0].style.display = "flex";
        review_model_button[0].style.backgroundColor = "rgb(229 231 235)";
        code_model_button[0].style.backgroundColor = "rgb(248 250 252)";
      })
      code_model_button[0].addEventListener('click', function () {
        show_code[0].style.display = "block";
        code_review_timeline[0].style.display = "none";
        review_model_button[0].style.backgroundColor = "rgb(248 250 252)";
        code_model_button[0].style.backgroundColor = "rgb(229 231 235)";
      })
    }
  })

  useEffect(() => {
    if (typeof window !== "undefined") {
      var return_to_top = document.getElementById("return_to_top");
      window.onscroll = function () {
        if (window.pageYOffset > 200) {
          return_to_top.style.display = "block";
        } else {
          setIs
          return_to_top.style.display = "none";
        }
      }
      return_to_top.addEventListener('click', function () {
        var timeId = setInterval(function () {
          var scrollTop = document.documentElement.scrollTop;
          if (scrollTop <= 0) {
            clearInterval(timeId);
          } else {
            window.scroll(0, scrollTop - 90);
          }
        }, 10)
      })
    }
  }, []);

  const [dir_file_data, setdir_file_data] = useState(null);
  const [readmeContent, setReadmeContent] = useState(null);
  const [sub_file_dir, setsub_file_dir] = useState(null);
  const [is_first_load, setis_first_load] = useState(true);
  const [click_history_content, setclick_history_content] = useState([]);
  const [current_sub_nodes, setcurrent_sub_nodes] = useState(null);
  const [clicked_nodeId, setclicked_nodeId] = useState(null);
  const [tree_view_key, settree_view_key] = useState(0);
  const [is_current_folder, setis_current_folder] = useState(false);
  // console.log(click_history);
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

  //获取项目的root目录

  useEffect(() => {
    async function get_file_dir() {
      try {
        const root_response = await axios.get(`/api/v1/tree?repo_path=/root/mega`);
        setdir_file_data(root_response.data);
        const index_clicked_node = root_response.data.items.find(item => item.id === itemId);
        let response;
        if (itemType === 'file') {
          console.log(itemType);
          handle_file_click(index_clicked_node.id, index_clicked_node.name);
        } else {
          response = await axios.get(`/api/v1/tree?repo_path=/root/mega&object_id=${itemId}`);
          setcurrent_sub_nodes(response.data);
        }
        console.log(index_clicked_node.name);
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
        if (is_first_load) {
          setsub_file_dir(response.data);
          setclick_history_content([{ id: "root", name: "MEGA" }]);
          setclick_history_content((prevHistory) => [
            ...prevHistory,
            { id: index_clicked_node.id, name: index_clicked_node.name, isFile: false }, // 添加 isFile 属性以区分文件和文件夹
          ]);
        }
      } catch (error) {
        console.error(error);
      }
    }
    get_file_dir();
  }, []);

  //获取项目的子目录, 且扫描子目录下是否有readme文件
  const get_sub_directory_data = async (directory_name, directoryId, item) => {
    try {
      const response = await axios.get(`/api/v1/tree?repo_path=/root/mega&object_id=${directoryId}`);
      setcurrent_sub_nodes(response.data);
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
        setReadmeContent(false);
      }
      return response.data;
    } catch (error) {
      console.error('Error fetching directory data:', error);
      return null;
    }
  };


  // dir点击事件，处理节点展开和关闭


  const [is_table_node_clicked, setis_table_node_clicked] = useState(false);
  const [expandedNodes, setExpandedNodes] = useState({});

  const handle_node_toggle = async (node, nodeId) => {
    setis_first_load(false);
    setis_file_clicked(false);
    setis_node_clicked(true);

    const newExpandedNodes = { ...expandedNodes };
    let parentNode = node.parent;
    while (parentNode) {
      newExpandedNodes[parentNode.id] = true;
      parentNode = parentNode.parent;
    }

    // 如果节点是目录，则进行加载
    if (node && node.content_type === 'directory') {
      if (expandedNodes[nodeId]) {
        setExpandedNodes(prevExpandedNodes => ({
          ...prevExpandedNodes,
          [nodeId]: false
        }));
      } else {
        // 展开节点及其所有父节点
        const newExpandedNodes = { ...expandedNodes };
        const expandNodeAndParents = (currentNode) => {
          newExpandedNodes[currentNode.id] = true;
          if (currentNode.parentId) {
            const parent = dir_file_data.items.find(item => item.id === currentNode.parentId);
            if (parent) {
              expandNodeAndParents(parent);
            }
          }
        };
        expandNodeAndParents(node);
        setExpandedNodes(newExpandedNodes);
        const newChildren = await get_sub_directory_data(node.name, node.id, node);
        if (newChildren) {
          node.children = newChildren.items;
          setsub_file_dir(newChildren);
          setdir_file_data({ ...dir_file_data }); // 更新状态
        }
      }
    }
  };

  const handle_sub_table_click = async (item) => {
    console.log("test items type");
    console.log(item);
    setclicked_nodeId(item.id);
    settree_view_key((prevKey) => prevKey + 1);
    if (item.content_type === "file") {
      handle_file_click(item.id, item.name);
      setclick_history_content((prevHistory) => [
        ...prevHistory,
        { id: item.id, name: item.name, isFile: true }, // 添加 isFile 属性以区分文件和文件夹
      ]);
    } else {
      handle_node_toggle(item, item.id);
      const newChildren = await get_sub_directory_data(item.name, item.id, item);
      setsub_file_dir(newChildren);
      setis_table_node_clicked(true); //当列表的文件夹被点击时，设置状态为true, 使得tree组件重新渲染，模拟节点点击逻辑
      setclick_history_content((prevHistory) => [
        ...prevHistory,
        { id: item.id, name: item.name, isFile: false },
      ]);
      var clickA = new Event("clickA");
      document.dispatchEvent(clickA);
      var tree_items = document.getElementsByClassName("MuiTreeItem-root");
      // console.log(click_history_content);

    }
  };

  const handle_level_return = async () => {
    // console.log("进入 handle_level_return");
    if (click_history_content.length <= 1) {
      // 已经在根目录，不需要返回
      return;
    }
    const previous_item = click_history_content[click_history_content.length - 2];
    // console.log(previous_item);
    setis_current_folder(false);
    if (previous_item.id === "root") {
      const response = await axios.get('/api/v1/tree?repo_path=/root/mega');
      setsub_file_dir(response.data);
    } else {
      const newChildren = await get_sub_directory_data(previous_item.name, previous_item.id, previous_item);
      setsub_file_dir(newChildren);
      // console.log(newChildren);
    }
    // 从历史中移除当前项
    setclick_history_content(current => current.slice(0, -1));
  }

  const handle_breadcrumb_click = async (index) => {
    setis_file_clicked(false);
    setis_node_clicked(true);
    const clicked_history = click_history_content.slice(0, index + 1);
    setclick_history_content(clicked_history);

    if (index === 0) {
      // 处理根目录点击
      const response = await axios.get('/api/v1/tree?repo_path=/root/mega');
      setsub_file_dir(response.data);
    } else {
      const parent_item = clicked_history[index];
      console.log(parent_item);
      const newChildren = await get_sub_directory_data(parent_item.name, parent_item.id, parent_item);
      setsub_file_dir(newChildren);
    }
  };

  //处理file点击事件，获得文本内容
  const [file_content, setfile_content] = useState(null);
  const [fileMode, setfile_mode] = useState('javascript');
  const handle_file_click = async (fileId, fileName) => {
    try {
      setis_file_clicked(true);
      setis_node_clicked(false);
      setis_table_node_clicked(false);
      const response = await axios.get(`/api/v1/blob?repo_path=/root/mega&object_id=${fileId}`);
      setfile_content(response.data.row_data);
      // 获取文件扩展名
      const languageMode = getFileLanguageMode(fileName);
      if (languageMode) {
        setFileMode(languageMode);
      }
    } catch (error) {
      console.error('Error fetching file content:', error);
    }
  };

  var is_dir_expand = false;


  // 重写 renderTree 组件
  const renderTree = (nodes, clicked_nodeId) => (
    <>
      {
        nodes.map((node) => {
          is_dir_expand = false;
          const { id, name, content_type } = node;
          if (node.content_type === "file") {
            is_dir_expand = false;
          } else {
            is_dir_expand = true;
          }
          const is_file = content_type === 'file';
          const isExpanded = expandedNodes[node.id];
          const is_clicked = clicked_nodeId === id;
          const handleNodeClick = () => {
            if (is_file) {
              // 如果是文件，处理文件点击事件
              handle_file_click(id, name);
            } else {
              // 如果是目录，处理目录点击事件
              handle_node_toggle(node, id);
              setclicked_nodeId(id);
            }
          };
          if (is_dir_expand) {
            return (
              <TreeItem
                key={id}
                nodeId={id}
                label={name}
                onClick={handleNodeClick}
                onLabelClick={() => handleNodeClick()}
              // style={nodeStyle} // Apply the style based on expanded state
              >
                {is_clicked && Array.isArray(node.children) && renderTree(node.children, clicked_nodeId)}
                <TreeItem />
              </TreeItem>
            );
          } else {
            return (
              <TreeItem
                key={id}
                nodeId={id}
                label={name}
                onClick={handleNodeClick}
              // style={nodeStyle} // Apply the style based on expanded state
              >
                {Array.isArray(node.children) ? renderTree(node.children) : null}
              </TreeItem>
            );
          }
        })}
    </>
  );

  return (
    <>
      <div className="h-100%  w-full full-windows-module">
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
        <div className="component_box_codemirror component_box box-content h-fit w-per p-center">
          <button className="return-top-button" id="return_to_top">^</button>
          <div className="component-left float-left  pr-5 w-1/5">
            <div className=" h-fit border-1">
              {dir_file_data && (
                <TreeView
                  key={tree_view_key} // 使用状态来触发重新渲染
                  defaultCollapseIcon={<ExpandMoreIcon />}
                  defaultExpandIcon={<ChevronRightIcon />}
                  onNodeToggle={handle_node_toggle}
                >
                  {renderTree(dir_file_data.items, clicked_nodeId)}
                </TreeView>
              )}
            </div>
          </div>

          <div className="component-right">
            <div className="Breadcrumb-div">
              <div role="presentation">
                <Breadcrumbs aria-label="breadcrumb">
                  {click_history_content.map((historyItem, index) => (
                    <React.Fragment key={index}>
                      {historyItem.isFile ? (
                        <Typography color="text.primary">
                          {historyItem.name}
                        </Typography>
                      ) : (
                        <Link
                          underline={index === click_history_content.length - 1 ? "none" : "hover"}
                          color={index === click_history_content.length - 1 ? "text.primary" : "inherit"}
                          onClick={index === click_history_content.length - 1 ? null : () => handle_breadcrumb_click(index)}
                        >
                          {historyItem.name}
                        </Link>
                      )}
                    </React.Fragment>
                  ))}
                </Breadcrumbs>
              </div>
            </div>
            <div className="module_border h-per w-4/5 bg-color ml-auto ">
              {is_file_clicked && (
                <span className="isolate inline-flex rounded-md shadow-sm ">
                  <button
                    type="button"
                    className="code-model-button model-button relative inline-flex items-center rounded-l-md bg-slate-50  py-2 text-sm font-semibold text-gray-900 ring-1 ring-inset ring-gray-300 hover:bg-gray-200 focus:z-10"
                  >
                    Code
                  </button>
                  <button
                    type="button"
                    className="review-model-button model-button relative -ml-px inline-flex items-center rounded-r-md bg-slate-50 px-3 py-2 text-sm font-semibold text-gray-900 ring-1 ring-inset ring-gray-300 hover:bg-gray-200 focus:z-10"
                  >
                    Blame
                  </button>
                </span>
              )}
            </div>
            <div className="view-window">
              <div className="show-code border-1">
                {
                  is_file_clicked && (
                    <CodeMirror
                      value={file_content}
                      options={{
                        lineNumbers: true,
                        abSize: 8,
                        theme: 'solarized',
                        scrollbarStyle: 'overlay',
                        lineWrapping: true,
                        readOnly: true,
                        foldGutter: true,
                        gutters: ["CodeMirror-linenumbers", "CodeMirror-foldgutter"]
                      }}
                      key={fileMode}
                      editorDidMount={(editor) => {
                        setshow_buttons(true);
                        const handleButtonClick = (e) => {
                          const lexicalInput = document.getElementsByClassName("review-Editor")[0];
                          lexicalInput.style.display = "flex";
                        };
                        if (show_buttons && is_file_clicked) {
                          for (let i = 0; i < editor.lineCount(); i++) {
                            const button = document.createElement("button");
                            button.textContent = "+";
                            button.className = "review_button";
                            button.addEventListener("click", handleButtonClick);

                            editor.setGutterMarker(i, "code-line-buttons", button);
                          }
                          editor.on("gutterClick", (cm, line, gutter, e) => {
                            if (gutter === "code-line-buttons") {
                              const lexicalInput = document.getElementsByClassName("review-Editor")[0];
                              lexicalInput.style.display = lexicalInput.style.display === "none" ? "flex" : "none";
                            }
                          });
                        }
                      }}
                    />
                  )
                }
                {is_node_clicked && (
                  <div className="sub-dir-show">
                    <div className="overflow-hidden shadow ring-1 ring-black ring-opacity-5 sm:rounded-lg">
                      <table className="min-w-full divide-y divide-gray-300">
                        {is_node_clicked && (
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
                                <a href='/' className='hover:text-blue-600'>&thinsp;777 commit</a>
                              </th>
                            </tr>
                          </thead>
                        )}
                        <tbody className="divide-y divide-gray-200 bg-white">
                          <tr className='flex'>
                            <button className="return_level_button"><svg t="1690163488952" className="flex float-left icon w-5 h-5 text-gray-400" viewBox="0 0 1024 1024" onClick={() => handle_level_return()}>
                              <path d="M81.16 412.073333L0 709.653333V138.666667a53.393333 53.393333 0 0 1 53.333333-53.333334h253.413334a52.986667 52.986667 0 0 1 37.713333 15.62l109.253333 109.253334a10.573333 10.573333 0 0 0 7.54 3.126666H842.666667a53.393333 53.393333 0 0 1 53.333333 53.333334v74.666666H173.773333a96.2 96.2 0 0 0-92.613333 70.74z m922-7.113333a52.933333 52.933333 0 0 0-42.386667-20.96H173.773333a53.453333 53.453333 0 0 0-51.453333 39.333333L11.773333 828.666667a53.333333 53.333333 0 0 0 51.453334 67.333333h787a53.453333 53.453333 0 0 0 51.453333-39.333333l110.546667-405.333334a52.953333 52.953333 0 0 0-9.073334-46.373333z" fill="#515151" p-id="16616"></path>
                            </svg>&nbsp;&nbsp;..</button>
                          </tr>
                          {sub_file_dir && sub_file_dir.items.map((item) => (
                            <tr key={item.name}>
                              <td className="flex whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6">
                                {get_icon_for_content_type(item.content_type)}
                                &ensp;
                                <a className='file_dir_link' onClick={() => handle_sub_table_click(item)} >{item.name}</a>
                              </td>
                              <td className="whitespace-nowrap px-3 py-4 text-sm text-gray-500"></td>
                              <td className="whitespace-nowrap pl-7 py-4 text-sm text-gray-500"></td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>

                  </div>
                )}

              </div>

              <div className=" code-review-timeline">
                <ul role="list" className="-mb-8">
                  {timeline.map((event, eventIdx) => (
                    <li key={event.id}>
                      <div className="relative pb-8">
                        {eventIdx !== timeline.length - 1 ? (
                          <span className="absolute left-4 top-4 -ml-px h-full w-0.5 bg-gray-200" aria-hidden="true" />
                        ) : null}
                        <div className="relative flex space-x-3">
                          <div>
                            <span
                              className={classNames(
                                event.iconBackground,
                                'h-8 w-8 rounded-full flex items-center justify-center ring-8 ring-white'
                              )}
                            >
                              <event.icon className="h-5 w-5 text-white" aria-hidden="true" />
                            </span>
                          </div>
                          <div className="flex min-w-0 flex-1 justify-between space-x-4 pt-3">
                            <div>
                              <p className="text-sm text-gray-500">
                                {event.content}{' '}
                                <a href={event.href} className="font-medium text-gray-900">
                                  {event.target}
                                </a>
                              </p>
                            </div>
                            <div className="whitespace-nowrap text-right text-sm text-gray-500">
                              <time dateTime={event.datetime}>{event.date}</time>
                            </div>
                          </div>
                        </div>
                      </div>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          </div>
          <div>
            {
              readmeContent && is_node_clicked && (
                <div className='the_markdown_content_sub'>
                  <div className='markdown_content_nav'>
                    <a onClick={() => router.push("/codeViewComponent")}>README.md</a>
                  </div>
                  <div className="markdown-body">
                    <ReactMarkdown>{readmeContent}</ReactMarkdown>
                  </div>
                </div>
              )}
          </div>
          <div className="review-Editor">
            <Editor />
          </div>
        </div>
      </div>
    </>
  );
}