import './App.css';
import "./output.css"
import { CheckIcon, HandThumbUpIcon, UserIcon } from '@heroicons/react/20/solid';
import React, { useState, useEffect } from "react";
import Editor from "./Editor.js";
import "./index.css";
import {UnControlled as CodeMirror} from 'react-codemirror2'
import 'codemirror/lib/codemirror.js'
import 'codemirror/lib/codemirror.css'
import './solarized.css'
import 'codemirror/mode/clike/clike'   
import 'codemirror/mode/javascript/javascript'  
import 'codemirror/addon/selection/active-line';
import 'codemirror/addon/fold/foldgutter.css';
import 'codemirror/addon/fold/foldcode.js';
import 'codemirror/addon/fold/foldgutter.js';
import 'codemirror/addon/fold/brace-fold.js';
import 'codemirror/addon/fold/xml-fold.js';
import 'codemirror/addon/fold/indent-fold.js';
import 'codemirror/addon/fold/markdown-fold.js';
import 'codemirror/addon/fold/comment-fold.js';
import 'codemirror/addon/scroll/simplescrollbars.js'
import 'codemirror/addon/scroll/simplescrollbars.css'
import { Tree, Input } from '@douyinfe/semi-ui';
import { Breadcrumb } from '@douyinfe/semi-ui';

//静态测试数据
const code_content = `var test = require('tape');
var dragula = require('..');

test('cancel does not throw when not dragging', function (t) {
  t.test('a single time', function once (st) {
    var drake = dragula();
    st.doesNotThrow(function () {
      drake.cancel();
    }, 'dragula ignores a single call to drake.cancel');
    st.end();
  });
  t.test('multiple times', function once (st) {
    var drake = dragula();
    st.doesNotThrow(function () {
      drake.cancel();
      drake.cancel();
      drake.cancel();
      drake.cancel();
    }, 'dragula ignores multiple calls to drake.cancel');
    st.end();
  });
  t.end();
});

test('when dragging and cancel gets called, nothing happens', function (t) {
  var div = document.createElement('div');
  var item = document.createElement('div');
  var drake = dragula([div]);
  div.appendChild(item);
  document.body.appendChild(div);
  drake.start(item);
  drake.cancel();
  t.equal(div.children.length, 1, 'nothing happens');
  t.equal(drake.dragging, false, 'drake has stopped dragging');
  t.end();
});

test('when dragging and cancel gets called, cancel event is emitted', function (t) {
  var div = document.createElement('div');
  var item = document.createElement('div');
  var drake = dragula([div]);
  div.appendChild(item);
  document.body.appendChild(div);
  drake.start(item);
  drake.on('cancel', cancel);
  drake.on('dragend', dragend);
  drake.cancel();
  t.plan(3);
  t.end();
  function dragend () {
    t.pass('dragend got called');
  }
  function cancel (target, container) {
    t.equal(target, item, 'cancel was invoked with item');
    t.equal(container, div, 'cancel was invoked with container');
  }
});

test('when dragging a copy and cancel gets called, default does not revert', function (t) {
  var div = document.createElement('div');
  var div2 = document.createElement('div');
  var item = document.createElement('div');
  var drake = dragula([div, div2]);
  div.appendChild(item);
  document.body.appendChild(div);
  document.body.appendChild(div2);
  drake.start(item);
  div2.appendChild(item);
  drake.on('drop', drop);
  drake.on('dragend', dragend);
  drake.cancel();
  t.plan(4);
  t.end();
  function dragend () {
    t.pass('dragend got called');
  }
  function drop (target, parent, source) {
    t.equal(target, item, 'drop was invoked with item');
    t.equal(parent, div2, 'drop was invoked with final container');
    t.equal(source, div, 'drop was invoked with source container');
  }
});

test('when dragging a copy and cancel gets called, revert is executed', function (t) {
  var div = document.createElement('div');
  var div2 = document.createElement('div');
  var item = document.createElement('div');
  var drake = dragula([div, div2]);
  div.appendChild(item);
  document.body.appendChild(div);
  document.body.appendChild(div2);
  drake.start(item);
  div2.appendChild(item);
  drake.on('cancel', cancel);
  drake.on('dragend', dragend);
  drake.cancel(true);
  t.plan(3);
  t.end();
  function dragend () {
    t.pass('dragend got called');
  }
  function cancel (target, container) {
    t.equal(target, item, 'cancel was invoked with item');
    t.equal(container, div, 'cancel was invoked with container');
  }
});
`;

const treeData = [
  {
      label: 'Asia',
      value: 'Asia',
      key: '0',
      children: [
          {
              label: 'China',
              value: 'China',
              key: '0-0',
              children: [
                  {
                      label: 'Beijing',
                      value: 'Beijing',
                      key: '0-0-0',
                  },
                  {
                      label: 'Shanghai',
                      value: 'Shanghai',
                      key: '0-0-1',
                  },
              ],
          },
          {
              label: 'Japan',
              value: 'Japan',
              key: '0-1',
              children: [
                  {
                      label: 'Osaka',
                      value: 'Osaka',
                      key: '0-1-0'
                  }
              ]
          },
      ],
  },
  {
      label: 'North America',
      value: 'North America',
      key: '1',
      children: [
          {
              label: 'United States',
              value: 'United States',
              key: '1-0'
          },
          {
              label: 'Canada',
              value: 'Canada',
              key: '1-1'
          }
      ]
  }
];

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
//------------

function classNames(...classes) {
  return classes.filter(Boolean).join(' ')
}
export default function Code_view(){
  let [CodeMirrorSize, setCodeMirrorSize] = useState(9);
  useEffect(() => {
    var review_div = document.getElementsByClassName("CodeMirror-line");
    var review_button = '<button style="" class="review_button" >+</button>';
    var lexical_input = document.getElementsByClassName("review-Editor");
    var CodeMirror_scroll = document.getElementsByClassName("CodeMirror-scroll");
    var CodeMirror_window = document.getElementsByClassName("CodeMirror");
    CodeMirror_window[0].style.height = "fit-content";
    CodeMirror_scroll[0].style.marginBottom = "0px";
    for(var i = 0; i < review_div.length; i ++){
      review_div[i].insertAdjacentHTML('afterBegin', review_button);
      review_div[i].style.zIndex = "100";
      var Review_Button = document.getElementsByClassName("review_button");
      Review_Button[i].addEventListener('click', function(){
        lexical_input[0].style.display = "flex";       
      })  
    }
  });

  useEffect(()=> {
    var return_to_top = document.getElementById("return_to_top");
    var code_review_timeline = document.getElementsByClassName("code-review-timeline");
    var show_code = document.getElementsByClassName("show-code");
    var review_model_button = document.getElementsByClassName("review-model-button");
    var code_model_button = document.getElementsByClassName("code-model-button");
    var Review_Canel_Button = document.getElementsByClassName("review_cancel_button");
    var Review_Post_Button = document.getElementsByClassName("review_post_button");
    var lexical_input = document.getElementsByClassName("review-Editor");
    review_model_button[0].addEventListener('click', function(){
      show_code[0].style.display = "none";
      code_review_timeline[0].style.display = "flex";
      review_model_button[0].style.backgroundColor = "rgb(229 231 235)";
      code_model_button[0].style.backgroundColor = "rgb(248 250 252)";
    })
    code_model_button[0].addEventListener('click', function(){
      show_code[0].style.display = "flex";
      code_review_timeline[0].style.display = "none";
      review_model_button[0].style.backgroundColor = "rgb(248 250 252)";
      code_model_button[0].style.backgroundColor = "rgb(229 231 235)";
    })
    window.onscroll = function(){
      if(window.pageYOffset > 200){
        return_to_top.style.display = "block";
      }else{
        return_to_top.style.display = "none";
      }
    }
    return_to_top.addEventListener('click',function(){
      var timeId = setInterval(function(){
          var scrollTop = document.documentElement.scrollTop;
          if(scrollTop <= 0){
              clearInterval(timeId);
          }else{
              scroll(0,scrollTop - 90);
          }
      },10)                                                 
    })
    Review_Post_Button[0].addEventListener('click', function(){
      lexical_input[0].style.display = "none";
      var editor_input = document.getElementsByClassName("editor-input");
    })
    Review_Canel_Button[0].addEventListener('click', function(){
      lexical_input[0].style.display = "none";
    })
    var fold_icon_element = document.getElementsByClassName("CodeMirror-sizer");
    const resizeObserver = new ResizeObserver(() => { 
      CodeMirrorSize += 1;
      setCodeMirrorSize(CodeMirrorSize);
    })
    resizeObserver.observe(fold_icon_element[0]);
    }, [])

  return (
    <> 
    <div className="h-100%  w-full ">
      <div className="component_box box-content h-fit w-per p-center pt-10">
      <button className="return-top-button" id = "return_to_top">^</button>  
        <div className="component-left float-left  pr-5 w-1/5 ">
            <div className=" h-fit border-1">
              <Tree
                filterTreeNode
                searchRender={({...restProps }) => (
                  <Input
                    {...restProps}
                  />
                )}
              treeData={treeData}
              />
            </div>
        </div>
        <div className="component-right">
          <div className=" border-1 h-per w-4/5 bg-color ml-auto">
            <span className="isolate inline-flex rounded-md shadow-sm ">
              <button
                type="button"
                className="code-model-button relative inline-flex items-center rounded-l-md bg-slate-50 px-3 py-2 text-sm font-semibold text-gray-900 ring-1 ring-inset ring-gray-300 hover:bg-gray-200 focus:z-10"
              >
                Code
              </button>
              <button
                type="button"
                className="review-model-button relative -ml-px inline-flex items-center rounded-r-md bg-slate-50 px-3 py-2 text-sm font-semibold text-gray-900 ring-1 ring-inset ring-gray-300 hover:bg-gray-200 focus:z-10"
              >
                Review
              </button>
            </span>
            <div className="Breadcrumb-div">
              <Breadcrumb>
                <Breadcrumb.Item ><span className="breadcrumb_text">level1</span></Breadcrumb.Item>
                <Breadcrumb.Item ><span className="breadcrumb_text">level2</span></Breadcrumb.Item>
                <Breadcrumb.Item ><span className="breadcrumb_text">level3</span></Breadcrumb.Item>
                <Breadcrumb.Item ><span className="breadcrumb_text">level4</span></Breadcrumb.Item>
              </Breadcrumb>
            </div>
          </div>
          <div className="view-window">
            <div className="show-code border-1">
              <CodeMirror
                value={code_content} 
                options = {{           
                    styleActiveLine:true,  
                    lineNumbers: true,
                    mode: { name: 'javascript', json: true },
                    styleActiveLine:true,
                    theme:'solarized',
                    scrollbarStyle:'overlay',
                    lineWrapping:true,
                    readOnly: true,
                    foldGutter:true,
                    gutters: ["CodeMirror-linenumbers", "CodeMirror-foldgutter"]
                }}
            />  
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
        <div className="review-Editor">
          <Editor />
        </div>
      </div>
    </div>
    </>
  )
}



