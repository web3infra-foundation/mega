import React from 'react';
import '../styles/TopNavbar.css';
import '../styles/globals.css';

const TopNavbar = () => {
    return (
        <nav>
            <div className='navContainer'>
                {/* 第一层 */}
                <div className='logo-container'>
                    <div className='navLeft'>
                        <img src="/images/mega.png" alt="log" className='logOfMega'></img>
                        <h1 className='titleOfMega'>MEGA</h1>
                    </div>
                    <div className='navRight'>
                        <div>
                            <div className="relative rounded-md shadow-sm navSearchContainer">
                                <div className="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 navSearchNextContainer">
                                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" className="w-6 h-6 text-gray-400">
                                        <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
                                    </svg>
                                </div>
                                <input
                                    type="email"
                                    name="email"
                                    id="email"
                                    className="navSerchInput block rounded-md border-0 text-gray-800 ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset  sm:text-sm sm:leading-6"
                                />
                            </div>
                        </div>
                        <a href='#'><img className='logOfNotification navLogs navRightItem' src='/icons/notification.svg' ></img></a>
                        <a href='#'><img className='navLogs navRightItem' src='/icons/help.svg' ></img></a>
                        <a href='#'><img className='logOfHead navRightItem' src='/icons/headImage.svg' ></img></a>
                    </div>
                </div>
                {/* 第二层的三个跳转按钮 */}
                <div className='navLinkIcons'>
                    <ul className='navLinkUl'>
                        <li className='navLinkLi'><img className='navPagesIcon' src="/icons/code.svg"></img><a href='#'>Code</a></li>
                        <li className='navLinkLi'><img className='navPagesIcon ' src="/icons/issues.svg"></img><a href='#'>Issues</a></li>
                        <li className='navLinkLi'><img className='navPagesIcon ' src="/icons/git-pull-request.svg"></img><a href='#'>Merge requests</a></li>
                    </ul>
                </div>
            </div >
        </nav >
    );
};

export default TopNavbar;
