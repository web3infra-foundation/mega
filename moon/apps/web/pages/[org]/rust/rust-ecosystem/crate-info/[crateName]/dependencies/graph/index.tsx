"use client";
import React, {  useState } from 'react';
import Head from 'next/head';
import { useParams } from 'next/navigation';
import { useRouter } from 'next/router';
import { AppLayout } from '@/components/Layout/AppLayout';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline';
import CrateInfoLayout from '../../layout';
import DependencyGraph from '../../../../../components/DependencyGraph';

const DependenciesGraphPage = () => {
    const params = useParams();
    const router = useRouter();
    const [currentPage, setCurrentPage] = useState(1);
    const [searchTerm, setSearchTerm] = useState('');

    // 从查询参数或URL参数中获取crate信息
    const crateName = (router.query.crateName as string) || params?.crateName as string || "tokio";
    // const version = (router.query.version as string) || params?.version as string || "1.2.01";
    const nsfront = params?.nsfront as string || router.query.org as string;

    const handleBackToTable = () => {
        router.push(`/${nsfront}/rust/rust-ecosystem/crate-info/${crateName}/dependencies`);
    };

    return (
        <>
            <Head>
                <title>Dependencies Graph - {crateName}</title>
            </Head>
            <CrateInfoLayout>
                {/* 主要内容区域 */}
                <div className="flex justify-center">
                    <div className="w-[1370px] px-8 py-4">
                        {/* 统一的白色面板 */}
                        <div className="bg-white rounded-lg shadow-sm border border-gray-200">
                            {/* 搜索和视图切换 - 在面板内部 */}
                            <div className="flex justify-between items-center p-2 border-b border-gray-200">
                                <div className="flex items-center flex-1 mr-4">
                                    <div className="relative w-full ml-2">
                                        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                                            <MagnifyingGlassIcon className="h-5 w-5 text-gray-400" />
                                        </div>
                                        <input
                                            type="text"
                                            placeholder="Placeholder"
                                            value={searchTerm}
                                            onChange={(e) => setSearchTerm(e.target.value)}
                                            style={{
                                                display: 'flex',
                                                height: 'var(--Spacing-8, 36px)',
                                                padding: '0 var(--Spacing-1, 4px)',
                                                alignItems: 'center',
                                                alignSelf: 'stretch',
                                                borderRadius: 'var(--Radius-2-max, 4px)',
                                                border: '1px solid var(--Colors-Neutral-Neutral-Alpha-5, #0009321f)',
                                                background: 'var(--Tokens-Colors-surface, #ffffffe6)',
                                                paddingLeft: '40px',
                                                width: '100%'
                                            }}
                                        />
                                    </div>
                                </div>
                                <div className="flex space-x-2 ml-auto mr-2">
                                    <button
                                        onClick={handleBackToTable}
                                        style={{
                                            display: 'flex',
                                            height: 'var(--Tokens-Space-button-height-2, 32px)',
                                            padding: '0 var(--Spacing-3, 12px)',
                                            justifyContent: 'center',
                                            alignItems: 'center',
                                            gap: 'var(--Spacing-2, 8px)',
                                            borderRadius: 'var(--Radius-2-max, 4px)',
                                            background: 'var(--Colors-Accent-Accent-Alpha-3, #0047f112)',
                                            color: '#002bb7c4',
                                            border: '1px solid var(--Colors-Neutral-Neutral-Alpha-5, #0009321f)'
                                        }}
                                    >
                                        <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                                            <path fillRule="evenodd" d="M3 4a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1V4zM3 10a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H4a1 1 0 01-1-1v-6zM14 9a1 1 0 00-1 1v6a1 1 0 001 1h2a1 1 0 001-1v-6a1 1 0 00-1-1h-2z" clipRule="evenodd" />
                                        </svg>
                                        <span>Table</span>
                                    </button>
                                    <button
                                        style={{
                                            display: 'flex',
                                            height: 'var(--Tokens-Space-button-height-2, 32px)',
                                            padding: '0 var(--Spacing-3, 12px)',
                                            justifyContent: 'center',
                                            alignItems: 'center',
                                            gap: 'var(--Spacing-2, 8px)',
                                            borderRadius: 'var(--Radius-2-max, 4px)',
                                            background: 'var(--Colors-Accent-Accent-9, #3E63DD)',
                                            color: 'white',
                                            border: 'none'
                                        }}
                                    >
                                        <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                                            <path d="M2 11a1 1 0 011-1h2a1 1 0 011 1v5a1 1 0 01-1 1H3a1 1 0 01-1-1v-5zM8 7a1 1 0 011-1h2a1 1 0 011 1v9a1 1 0 01-1 1H9a1 1 0 01-1-1V7zM14 4a1 1 0 011-1h2a1 1 0 011 1v12a1 1 0 01-1 1h-2a1 1 0 01-1-1V4z" />
                                        </svg>
                                        <span style={{
                                            fontFamily: '"SF Pro"',
                                            fontSize: '14px',
                                            fontStyle: 'normal',
                                            fontWeight: '500',
                                            lineHeight: '20px',
                                            letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                        }}>Graph</span>
                                    </button>
                                </div>
                            </div>

                            {/* 图形视图内容 */}
                            <div className="w-full h-full p-6" style={{ height: '100%', width: '100%' }}>
                                <DependencyGraph />
                            </div>
                        </div>

                        {/* 分页功能区 */}
                        <div className="w-full flex justify-center mt-8">
                            <div style={{ width: '1370px', paddingLeft: 32, paddingRight: 32 }}>
                                <div className="flex justify-center items-center gap-6" style={{ marginLeft: '-100px' }}>
                                    {/* Previous 按钮 */}
                                    <button
                                        onClick={() => setCurrentPage(Math.max(1, currentPage - 1))}
                                        disabled={currentPage === 1}
                                        className="flex items-center text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
                                    >
                                        <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                                        </svg>
                                        Previous
                                    </button>

                                    {/* 当前页码 */}
                                    <span className="text-lg font-bold text-gray-900 ml-2 mr-2" style={{ fontSize: '14px' }}>{currentPage}</span>

                                    {/* Next 按钮 */}
                                    <button
                                        onClick={() => setCurrentPage(Math.min(5, currentPage + 1))}
                                        disabled={currentPage === 5}
                                        className="flex items-center text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
                                    >
                                        Next
                                        <svg className="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                                        </svg>
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </CrateInfoLayout>
        </>
    );
};

// 添加 getProviders 方法以适配新的项目结构
DependenciesGraphPage.getProviders = (page: any, pageProps: any) => {
    return (
        <AuthAppProviders {...pageProps}>
            <AppLayout {...pageProps}>{page}</AppLayout>
        </AuthAppProviders>
    );
};

export default DependenciesGraphPage;
