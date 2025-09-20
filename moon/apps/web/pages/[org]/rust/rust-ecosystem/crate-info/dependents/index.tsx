"use client";
import React, { useEffect, useState } from 'react';
import Head from 'next/head';
import { useParams } from 'next/navigation';
import { useRouter } from 'next/router';
import { AppLayout } from '@/components/Layout/AppLayout';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
// import { MagnifyingGlassIcon } from '@heroicons/react/24/outline';
import CrateInfoLayout from '../layout';

interface Dependent {
    id: string;
    name: string;
    version: string;
    relation: 'Direct' | 'Indirect';
    license?: string;
    dependencies?: number;
    expanded?: boolean;
    description?: string;
    published?: string;
}

const DependentsPage = () => {
    const params = useParams();
    const router = useRouter();
    const [dependents, setDependents] = useState<Dependent[]>([]);
    const [currentPage, setCurrentPage] = useState(1);
    // const [viewMode, setViewMode] = useState<'table' | 'graph'>('table');
    const searchTerm = '';

    // 从查询参数或URL参数中获取crate信息
    const crateName = (router.query.crateName as string) || params?.crateName as string || "tokio";
    const version = (router.query.version as string) || params?.version as string || "1.2.01";
    // const nsfront = (router.query.nsfront as string) || params?.nsfront as string || router.query.org as string;
    // const nsbehind = (router.query.nsbehind as string) || params?.nsbehind as string || "rust/rust-ecosystem/crate-info";

    useEffect(() => {
        // 模拟dependents数据 - 显示使用当前包的其他包
        const mockDependents: Dependent[] = [
            {
                id: '1',
                name: 'github-random-star',
                version: '1.0.2',
                relation: 'Direct',
                expanded: false
            },
            {
                id: '2',
                name: 'github-random-star',
                version: '1.0.1',
                relation: 'Direct',
                expanded: false
            }
        ];

        // 直接设置数据，不使用加载延迟
        setDependents(mockDependents);
    }, [crateName, version]);

    const filteredDependents = dependents.filter(dep =>
        dep.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        dep.version.toLowerCase().includes(searchTerm.toLowerCase())
    );

    return (
        <>
            <Head>
                <title>Dependents - {crateName}</title>
            </Head>
            <CrateInfoLayout>
                {/* 主要内容区域 */}
                <div className="flex justify-center">
                    <div className="w-[1370px] px-8 py-4">
                        {/* 统一的白色面板 */}
                        <div className="bg-white rounded-lg shadow-sm border border-gray-200">


                            {/* 数据统计显示 - 在面板内部 */}
                            <div className="p-4 border-b border-gray-200">
                                                                <div className="flex items-center">
                                    <div className="flex flex-col space-y-2" style={{ marginLeft: '8px' }}>
                                        <span style={{
                                            display: '-webkit-box',
                                            WebkitBoxOrient: 'vertical',
                                            WebkitLineClamp: 1,
                                            overflow: 'hidden',
                                            color: '#1c2024',
                                            textOverflow: 'ellipsis',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '14px',
                                            fontStyle: 'normal',
                                            fontWeight: '400',
                                            lineHeight: '20px',
                                            letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                        }}>Direct</span>
                                        <span style={{
                                            display: '-webkit-box',
                                            WebkitBoxOrient: 'vertical',
                                            WebkitLineClamp: 1,
                                            overflow: 'hidden',
                                            color: '#1c2024',
                                            textOverflow: 'ellipsis',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '14px',
                                            fontStyle: 'normal',
                                            fontWeight: '400',
                                            lineHeight: '20px',
                                            letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                        }}>Indirect</span>
                                    </div>
                                    
                                    <div className="flex flex-col space-y-2 items-end ml-8" style={{ marginLeft: '600px' }}>
                                        <span style={{
                                            display: '-webkit-box',
                                            WebkitBoxOrient: 'vertical',
                                            WebkitLineClamp: 1,
                                            overflow: 'hidden',
                                            color: '#3e63dd',
                                            textOverflow: 'ellipsis',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '14px',
                                            fontStyle: 'normal',
                                            fontWeight: '400',
                                            lineHeight: '20px',
                                            letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                        }}>
                                            26
                                        </span>
                                        <span style={{
                                            display: '-webkit-box',
                                            WebkitBoxOrient: 'vertical',
                                            WebkitLineClamp: 1,
                                            overflow: 'hidden',
                                            color: '#3e63dd',
                                            textOverflow: 'ellipsis',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '14px',
                                            fontStyle: 'normal',
                                            fontWeight: '400',
                                            lineHeight: '20px',
                                            letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                        }}>
                                            12
                                        </span>
                                    </div>
                                    
                                    {/* 进度条 */}
                                    <div className="flex flex-col space-y-2 ml-4" style={{ width: '596px' }}>
                                        <div className="h-2 rounded-lg overflow-hidden" style={{ marginTop: '-2px', backgroundColor: 'rgb(238,238,241)' }}>
                                            <div
                                                className="h-full rounded-lg"
                                                style={{ width: '68%', backgroundColor: 'rgb(61,98,220)' }}
                                            />
                                        </div>
                                        <div className="h-2 rounded-lg overflow-hidden" style={{ marginTop: '18px', backgroundColor: 'rgb(238,238,241)' }}>
                                            <div
                                                className="h-full rounded-lg"   
                                                style={{ width: '32%', backgroundColor: 'rgb(61,98,220)' }}
                                            />
                                        </div>
                                    </div>
                                </div>
                            </div>

                            {/* 表格 - 在面板内部 */}
                            <div className="overflow-x-auto">
                                <table className="min-w-full divide-y divide-gray-200">
                                    <thead style={{ background: '#ffffff00' }}>
                                        <tr>
                                            <th className="px-6 py-3 text-left " style={{ marginRight : '20px', marginLeft: '-12px' }}>
                                                 <span style={{
                                                     display: '-webkit-box',
                                                     WebkitBoxOrient: 'vertical',
                                                     WebkitLineClamp: 1,
                                                     overflow: 'hidden',
                                                     color: '#1c2024',
                                                     textOverflow: 'ellipsis',
                                                     fontFamily: '"HarmonyOS Sans SC"',
                                                     fontSize: '14px',
                                                     fontStyle: 'normal',
                                                     fontWeight: '400',
                                                     lineHeight: '20px',
                                                     letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                 }}>Package</span>
                                             </th>
                                                <th className="px-6 py-3 text-right " style={{ paddingLeft: '300px' }}>
                                                    <span style={{
                                                        display: '-webkit-box',
                                                        WebkitBoxOrient: 'vertical',
                                                        WebkitLineClamp: 1,
                                                        overflow: 'hidden',
                                                        color: '#1c2024',
                                                        textOverflow: 'ellipsis',
                                                        fontFamily: '"HarmonyOS Sans SC"',
                                                        fontSize: '14px',
                                                        fontStyle: 'normal',
                                                        fontWeight: '400',
                                                        lineHeight: '20px',
                                                        letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                    }}>Version</span>
                                                </th>
                                             <th className="px-6 py-3 text-right ">
                                                 <span style={{
                                                     display: '-webkit-box',
                                                     WebkitBoxOrient: 'vertical',
                                                     WebkitLineClamp: 1,
                                                     overflow: 'hidden',
                                                     color: '#1c2024',
                                                     textOverflow: 'ellipsis',
                                                     fontFamily: '"HarmonyOS Sans SC"',
                                                     fontSize: '14px',
                                                     fontStyle: 'normal',
                                                     fontWeight: '400',
                                                     lineHeight: '20px',
                                                     letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                 }}>Relation</span>
                                             </th>
                                        </tr>
                                    </thead>
                                    <tbody className="bg-white divide-y divide-gray-200">
                                        {filteredDependents.map((dependent) => (
                                            <React.Fragment key={dependent.id}>
                                                <tr className="hover:bg-gray-50">
                                                    <td className="px-6 py-4 whitespace-nowrap" style={{ paddingLeft: '24px' }}>
                                                         <div 
                                                             className="cursor-pointer hover:underline"
                                                             style={{
                                                                 display: '-webkit-box',
                                                                 WebkitBoxOrient: 'vertical',
                                                                 WebkitLineClamp: 1,
                                                                 overflow: 'hidden',
                                                                 color: '#002bb7c4',
                                                                 textOverflow: 'ellipsis',
                                                                 fontFamily: '"HarmonyOS Sans SC"',
                                                                 fontSize: '14px',
                                                                 fontStyle: 'normal',
                                                                 fontWeight: 400,
                                                                 lineHeight: '20px',
                                                                 letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                             }}
                                                         >
                                                             {dependent.name}
                                                         </div>
                                                    </td>                                        
                                                      <td className="px-6 py-4 whitespace-nowrap text-right" style={{ paddingLeft: '300px' }}>
                                                              <span 
                                                                   className="cursor-pointer hover:underline"
                                                                   style={{
                                                                       display: '-webkit-box',
                                                                       WebkitBoxOrient: 'vertical',
                                                                       WebkitLineClamp: 1,
                                                                       overflow: 'hidden',
                                                                       color: '#1c2024',
                                                                       textOverflow: 'ellipsis',
                                                                       fontFamily: '"SF Pro"',
                                                                       fontSize: '14px',
                                                                       fontStyle: 'normal',
                                                                       fontWeight: 400,
                                                                       lineHeight: '20px',
                                                                       letterSpacing: 'var(--Typography-Letter-spacing-2, 0)',
                                                                       marginRight: '16px'
                                                                   }}
                                                               >
                                                                   {dependent.version}
                                                               </span>
                                                         </td>
                                                                <td className="px-6 py-4 whitespace-nowrap text-right">
                                                                                                                    <span 
                                                               className="cursor-pointer hover:underline"
                                                               style={{
                                                                   display: '-webkit-box',
                                                                   WebkitBoxOrient: 'vertical',
                                                                   WebkitLineClamp: 1,
                                                                   overflow: 'hidden',
                                                                   color: '#002bb7c4',
                                                                   textOverflow: 'ellipsis',
                                                                   fontFamily: '"HarmonyOS Sans SC"',
                                                                   fontSize: '14px',
                                                                   fontStyle: 'normal',
                                                                   fontWeight: 400,
                                                                   lineHeight: '20px',
                                                                   letterSpacing: 'var(--Typography-Letter-spacing-2, 0)',
                                                                   marginRight: '14px'
                                                               }}
                                                           >
                                                               {dependent.relation}
                                                           </span>
                                                      </td>
                                                </tr>
                                                {dependent.expanded && dependent.description && (
                                                    <tr className="bg-gray-50">
                                                        <td colSpan={3} className="px-6 py-4">
                                                            <div className="space-y-2">
                                                                <div className="flex items-center space-x-4">
                                                                    <span className="text-sm text-blue-600 hover:text-blue-800 cursor-pointer">
                                                                        Version: {dependent.version}
                                                                    </span>
                                                                    <span className="text-sm text-gray-500">
                                                                        Published: {dependent.published}
                                                                    </span>
                                                                </div>
                                                                <p className="text-sm text-gray-700">
                                                                    {dependent.description}
                                                                </p>
                                                            </div>
                                                        </td>
                                                    </tr>
                                                )}
                                            </React.Fragment>
                                        ))}
                                    </tbody>
                                </table>
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
DependentsPage.getProviders = (page: any, pageProps: any) => {
    return (
        <AuthAppProviders {...pageProps}>
            <AppLayout {...pageProps}>{page}</AppLayout>
        </AuthAppProviders>
    );
};

export default DependentsPage;
