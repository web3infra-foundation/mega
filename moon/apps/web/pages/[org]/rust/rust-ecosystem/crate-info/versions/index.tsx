"use client";
import React, { useEffect, useState } from 'react';
import Head from 'next/head';
import { useParams } from 'next/navigation';
import { useRouter } from 'next/router';
import { AppLayout } from '@/components/Layout/AppLayout';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
// import { ChevronUpDownIcon } from '@heroicons/react/24/outline';
import CrateInfoLayout from '../layout';
import Image from 'next/image';

interface Versionpage {
    version: string;
    updated_at: string;
    downloads: string;
    dependents: number;
}

interface VersionInfo extends Versionpage {
    id: string;
    published: string;
}

const VersionsPage = () => {
    const params = useParams();
    const router = useRouter();
    const [versions, setVersions] = useState<VersionInfo[]>([]);
    const [currentPage, setCurrentPage] = useState(1);
    const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const itemsPerPage = 10;

    // 从查询参数或URL参数中获取crate信息
    const crateName = (router.query.crateName as string) || params?.crateName as string || "tokio";
    const version = (router.query.version as string) || params?.version as string || "1.2.01";
    const nsfront = (router.query.nsfront as string) || params?.nsfront as string || router.query.org as string;
    const nsbehind = (router.query.nsbehind as string) || params?.nsbehind as string || "rust/rust-ecosystem/crate-info";

    // 从 API 获取版本数据
    useEffect(() => {
        const fetchVersions = async () => {
            if (!crateName || !version || !nsfront || !nsbehind) return;
            
            try {
                setLoading(true);
                setError(null);
                
                const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL;
                const response = await fetch(`${apiBaseUrl}/api/crates/${nsfront}/${nsbehind}/${crateName}/${version}/versions`);
                
                if (!response.ok) {
                    throw new Error('Failed to fetch versions');
                }
                
                const data: Versionpage[] = await response.json();
                
                // 转换 API 数据为前端需要的格式
                const transformedVersions: VersionInfo[] = data.map((ver, index) => ({
                    id: `${ver.version}-${index}`,
                    version: ver.version,
                    published: ver.updated_at,
                    dependents: ver.dependents,
                    updated_at: ver.updated_at,
                    downloads: ver.downloads
                }));

                // 排序版本
                const sortedVersions = [...transformedVersions].sort((a, b) => {
                    const aVersion = a.version.split('.').map(Number);
                    const bVersion = b.version.split('.').map(Number);
                    
                    for (let i = 0; i < Math.max(aVersion.length, bVersion.length); i++) {
                        const aPart = aVersion[i] || 0;
                        const bPart = bVersion[i] || 0;
                        
                        if (aPart !== bPart) {
                            return sortOrder === 'desc' ? bPart - aPart : aPart - bPart;
                        }
                    }
                    return 0;
                });

                setVersions(sortedVersions);
            } catch (err) {
                setError('Failed to load versions');
            } finally {
                setLoading(false);
            }
        };

        fetchVersions();
    }, [crateName, version, nsfront, nsbehind, sortOrder]);

    const handleSort = () => {
        setSortOrder(sortOrder === 'desc' ? 'asc' : 'desc');
    };

    const handleVersionClick = (_version: string) => {
        // 可以导航到该版本的详情页
        // TODO: 实现版本导航功能
    };

    // 分页逻辑
    const totalPages = Math.ceil(versions.length / itemsPerPage);
    const startIndex = (currentPage - 1) * itemsPerPage;
    const endIndex = startIndex + itemsPerPage;
    const currentVersions = versions.slice(startIndex, endIndex);

    const handlePreviousPage = () => {
        if (currentPage > 1) {
            setCurrentPage(currentPage - 1);
        }
    };

    const handleNextPage = () => {
        if (currentPage < totalPages) {
            setCurrentPage(currentPage + 1);
        }
    };

    return (
        <>
            <Head>
                <title>Versions - {crateName}</title>
            </Head>
            <CrateInfoLayout>
                <div className="flex justify-center">
                    <div className="w-[1370px] px-8 py-4">
                        {/* 加载状态 */}
                        {loading && (
                            <div className="flex justify-center items-center py-8">
                                <div className="text-gray-500">Loading versions...</div>
                            </div>
                        )}
                        
                        {/* 错误状态 */}
                        {error && (
                            <div className="flex justify-center items-center py-8">
                                <div className="text-red-500">{error}</div>
                            </div>
                        )}

                        {/* 表格 */}
                        {!loading && !error && (
                            <div className="bg-white rounded-lg shadow-sm border border-gray-200">
                                <div className="overflow-x-auto">
                                    <table className="min-w-full divide-y divide-gray-200">
                                    <thead style={{ background: 'rgb(241,241,245)' }}>
                                        <tr>
                                            <th className="px-6 py-3 text-left w-1/3">
                                                <button
                                                    onClick={handleSort}
                                                    className="flex items-center space-x-1"
                                                >
                                                                                                         <span style={{
                                                         display: '-webkit-box',
                                                         WebkitBoxOrient: 'vertical',
                                                         WebkitLineClamp: 1,
                                                         overflow: 'hidden',
                                                         color: '#1c2024',
                                                         textOverflow: 'ellipsis',
                                                         fontFamily: '"SF Pro"',
                                                         fontSize: '14px',
                                                         fontStyle: 'normal',
                                                         fontWeight: '400',
                                                         lineHeight: '20px',
                                                         letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                     }}>Version</span>
                                                     <Image     
                                                         src="/rust/rust-ecosystem/crate-info/dependencies/double-arrow-up.png" 
                                                         alt="sort" 
                                                         className="w-4 h-4"
                                                         width={4}
                                                         height={4}
                                                         style={{ transform: 'rotate(180deg)', marginLeft: '8px' }}
                                                     />
                                                </button>
                                            </th>
                                            <th className="px-6 py-3 text-left w-1/3">
                                                                                                 <span style={{
                                                     display: '-webkit-box',
                                                     WebkitBoxOrient: 'vertical',
                                                     WebkitLineClamp: 1,
                                                     overflow: 'hidden',
                                                     color: '#1c2024',
                                                     textOverflow: 'ellipsis',
                                                     fontFamily: '"SF Pro"',
                                                     fontSize: '14px',
                                                     fontStyle: 'normal',
                                                     fontWeight: '500',
                                                     lineHeight: '20px',
                                                     letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                 }}>Published</span>
                                            </th>
                                            <th className="px-6 py-3 text-left w-1/3">
                                                                                                 <span style={{
                                                     display: '-webkit-box',
                                                     WebkitBoxOrient: 'vertical',
                                                     WebkitLineClamp: 1,
                                                     overflow: 'hidden',
                                                     color: '#1c2024',
                                                     textOverflow: 'ellipsis',
                                                     fontFamily: '"SF Pro"',
                                                     fontSize: '14px',
                                                     fontStyle: 'normal',
                                                     fontWeight: '500',
                                                     lineHeight: '20px',
                                                     letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                 }}>Dependents</span>
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody className="bg-white divide-y divide-gray-200">
                                        {currentVersions.map((versionInfo) => (
                                            <tr key={versionInfo.id} className="hover:bg-gray-50">
                                                <td className="px-6 py-4 whitespace-nowrap">
                                                    <button
                                                        onClick={() => handleVersionClick(versionInfo.version)}
                                                        className="cursor-pointer hover:underline"
                                                                                                                 style={{
                                                             display: '-webkit-box',
                                                             WebkitBoxOrient: 'vertical',
                                                             WebkitLineClamp: 1,
                                                             overflow: 'hidden',
                                                             color: '#002bb7c4',
                                                             textOverflow: 'ellipsis',
                                                             fontFamily: '"SF Pro"',
                                                             fontSize: '14px',
                                                             fontStyle: 'normal',
                                                             fontWeight: 400,
                                                             lineHeight: '20px',
                                                             letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                         }}
                                                    >
                                                        {versionInfo.version}
                                                    </button>
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap">
                                                                                                         <span 
                                                         style={{
                                                             display: '-webkit-box',
                                                             WebkitBoxOrient: 'vertical',
                                                             WebkitLineClamp: 1,
                                                             overflow: 'hidden',
                                                             color: '#80838d',
                                                             textOverflow: 'ellipsis',
                                                             fontFamily: '"SF Pro"',
                                                             fontSize: '14px',
                                                             fontStyle: 'normal',
                                                             fontWeight: 400,
                                                             lineHeight: '20px',
                                                             letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                         }}
                                                     >
                                                         {versionInfo.published}
                                                     </span>
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap">
                                                                                                         <span 
                                                         style={{
                                                             display: '-webkit-box',
                                                             WebkitBoxOrient: 'vertical',
                                                             WebkitLineClamp: 1,
                                                             overflow: 'hidden',
                                                             color: '#80838d',
                                                             textOverflow: 'ellipsis',
                                                             fontFamily: '"SF Pro"',
                                                             fontSize: '14px',
                                                             fontStyle: 'normal',
                                                             fontWeight: 400,
                                                             lineHeight: '20px',
                                                             letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                         }}
                                                     >
                                                         {versionInfo.dependents.toLocaleString()}
                                                     </span>
                                                </td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                                </div>
                            </div>
                        )}

                        {/* 分页功能区 */}
                        <div className="w-full flex justify-center mt-8">
                            <div style={{ width: '1370px', paddingLeft: 32, paddingRight: 32 }}>
                                <div className="flex justify-center items-center gap-6" style={{ marginLeft: '-100px' }}>
                                    {/* Previous 按钮 */}
                                    <button
                                        onClick={handlePreviousPage}
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
                                        onClick={handleNextPage}
                                        disabled={currentPage === totalPages}
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
VersionsPage.getProviders = (page: any, pageProps: any) => {
    return (
        <AuthAppProviders {...pageProps}>
            <AppLayout {...pageProps}>{page}</AppLayout>
        </AuthAppProviders>
    );
};

export default VersionsPage;
