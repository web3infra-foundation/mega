"use client";
import React, { useEffect, useState } from 'react';
import Head from 'next/head';
import { useParams } from 'next/navigation';
import { useRouter } from 'next/router';
import { AppLayout } from '@/components/Layout/AppLayout';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import { MagnifyingGlassIcon, ChevronDownIcon, ChevronUpIcon } from '@heroicons/react/24/outline';
import CrateInfoLayout from '../layout';
import Image from 'next/image';

interface DependencyCrateInfo {
    crate_name: string;
    version: string;
    relation: string;
    license: string;
    dependencies: number;
}

interface DependencyInfo {
    direct_count: number;
    indirect_count: number;
    data: DependencyCrateInfo[];
}

interface Dependency extends DependencyCrateInfo {
    id: string;
    expanded?: boolean;
    description?: string;
    published?: string;
}

const DependenciesPage = () => {
    const params = useParams();
    const router = useRouter();
    const [dependencies, setDependencies] = useState<Dependency[]>([]);
    const [currentPage, setCurrentPage] = useState(1);
    const [searchTerm, setSearchTerm] = useState('');
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    // 从查询参数或URL参数中获取crate信息
    const crateName = (router.query.crateName as string) || params?.crateName as string || "tokio";
    const version = (router.query.version as string) || params?.version as string || "1.2.01";
    const nsfront = (router.query.nsfront as string) || params?.nsfront as string || router.query.org as string;
    const nsbehind = (router.query.nsbehind as string) || params?.nsbehind as string || "rust/rust-ecosystem/crate-info";

    // 从 API 获取依赖数据
    useEffect(() => {
        const fetchDependencies = async () => {
            if (!crateName || !version || !nsfront || !nsbehind) return;
            
            try {
                setLoading(true);
                setError(null);
                
                const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL;
                const response = await fetch(`${apiBaseUrl}/api/crates/${nsfront}/${nsbehind}/${crateName}/${version}/dependencies`);
                
                if (!response.ok) {
                    throw new Error('Failed to fetch dependencies');
                }
                
                const data: DependencyInfo = await response.json();
                
                // 转换 API 数据为前端需要的格式
                const transformedDependencies: Dependency[] = data.data.map((dep, index) => ({
                    id: `${dep.crate_name}-${dep.version}-${index}`,
                    crate_name: dep.crate_name,
                    version: dep.version,
                    relation: dep.relation as 'Direct' | 'Indirect',
                    license: dep.license,
                    dependencies: dep.dependencies,
                    expanded: false,
                    description: `Dependency for ${dep.crate_name}`,
                    published: 'Unknown'
                }));
                
                setDependencies(transformedDependencies);
            } catch (err) {
                setError('Failed to load dependencies');
            } finally {
                setLoading(false);
            }
        };

        fetchDependencies();
    }, [crateName, version, nsfront, nsbehind]);

    const toggleExpanded = (id: string) => {
        setDependencies(prev =>
            prev.map(dep =>
                dep.id === id ? { ...dep, expanded: !dep.expanded } : dep
            )
        );
    };

    const filteredDependencies = dependencies.filter(dep =>
        dep.crate_name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        dep.version.toLowerCase().includes(searchTerm.toLowerCase())
    );

    const handleNavigateToGraph = () => {
        router.push({
            pathname: `/${nsfront}/rust/rust-ecosystem/crate-info/dependencies/graph`,
            query: {
                crateName: crateName,
                version: version,
                nsfront: nsfront,
                nsbehind: nsbehind
            }
        });
    };

    return (
        <>
            <Head>
                <title>Dependencies - {crateName}</title>
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
                                            <path fillRule="evenodd" d="M3 4a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1V4zM3 10a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H4a1 1 0 01-1-1v-6zM14 9a1 1 0 00-1 1v6a1 1 0 001 1h2a1 1 0 001-1v-6a1 1 0 00-1-1h-2z" clipRule="evenodd" />
                                        </svg>
                                        <span>Table</span>
                                    </button>
                                    <button
                                        onClick={handleNavigateToGraph}
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

                            {/* 加载状态 */}
                            {loading && (
                                <div className="flex justify-center items-center py-8">
                                    <div className="text-gray-500">Loading dependencies...</div>
                                </div>
                            )}
                            
                            {/* 错误状态 */}
                            {error && (
                                <div className="flex justify-center items-center py-8">
                                    <div className="text-red-500">{error}</div>
                                </div>
                            )}

                            {/* 表格视图内容 */}
                            {!loading && !error && (
                                <div className="overflow-x-auto">
                                    <table className="min-w-full divide-y divide-gray-200">
                                    <thead style={{ background: '#ffffff00' }}>
                                        <tr>
                                            <th className="px-6 py-3 text-left w-1/3">
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
                                            <th className="px-6 py-3 text-right w-1/6">
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
                                                }}>Notes</span>
                                            </th>
                                            <th className="px-6 py-3 text-left w-1/6" style={{ paddingLeft: 'calc(1.5rem + 90px)' }}>
                                                <div className="flex items-center space-x-1">
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
                                                    <Image 
                                                        src="/rust/rust-ecosystem/crate-info/dependencies/double-arrow-up.png" 
                                                        alt="sort" 
                                                        className="w-4 h-4"
                                                        width={4}
                                                        height={4}
                                                    />
                                                </div>
                                            </th>
                                            <th className="px-6 py-3 text-left w-1/6" style={{ paddingLeft: 'calc(1.5rem + 90px)' }}>
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
                                                }}>License</span>
                                            </th>
                                            <th className="px-6 py-3 text-left w-1/6" style={{ paddingLeft: 'calc(1.5rem + 68px)' }}>
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
                                                }}>Dependencies</span>
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody className="bg-white divide-y divide-gray-200">
                                        {filteredDependencies.map((dependency) => (
                                            <React.Fragment key={dependency.id}>
                                                <tr className="hover:bg-gray-50">
                                                    <td className="px-6 py-4 whitespace-nowrap">
                                                        <div className="flex items-center space-x-2">
                                                            <button
                                                                onClick={() => toggleExpanded(dependency.id)}
                                                                className="text-gray-400 hover:text-gray-600"
                                                            >
                                                                {dependency.expanded ? (
                                                                    <ChevronUpIcon className="w-4 h-4" />
                                                                ) : (
                                                                    <ChevronDownIcon className="w-4 h-4" />
                                                                )}
                                                            </button>
                                                            <div>
                                                                <div className="text-sm font-medium text-gray-900">
                                                                    {dependency.crate_name}
                                                                </div>
                                                                <div className="text-sm text-gray-500">
                                                                    {dependency.version}
                                                                </div>
                                                            </div>
                                                        </div>
                                                    </td>
                                                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 text-right">
                                                     
                                                    </td>
                                                    <td className="px-6 py-4 whitespace-nowrap text-left" style={{ paddingLeft: 'calc(1.5rem + 91px)' }}>
                                                        <span style={{
                                                            display: '-webkit-box',
                                                            WebkitBoxOrient: 'vertical',
                                                            WebkitLineClamp: 1,
                                                            overflow: 'hidden',
                                                            color: '#002bb7c4',
                                                            textOverflow: 'ellipsis',
                                                            fontFamily: '"HarmonyOS Sans SC"',
                                                            fontSize: '14px',
                                                            fontStyle: 'normal',
                                                            fontWeight: '400',
                                                            lineHeight: '20px',
                                                            letterSpacing: 'var(--Typography-Letter-spacing-2, 0)',
                                                            cursor: 'pointer'
                                                        }}>
                                                            {dependency.relation}
                                                        </span>
                                                    </td>
                                                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900 text-left" style={{ paddingLeft: 'calc(1.5rem + 91px)' }}>
                                                        {dependency.license}
                                                    </td>
                                                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900 text-left" style={{ paddingLeft: 'calc(1.5rem + 68px)' }}>
                                                        {dependency.dependencies}
                                                    </td>
                                                </tr>
                                                {dependency.expanded && dependency.description && (
                                                    <tr className="bg-gray-50">
                                                        <td colSpan={5} className="px-6 py-4">
                                                            <div className="ml-8">
                                                                <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                                                                    <tbody>
                                                                        <tr>
                                                                            <td style={{ 
                                                                                width: '120px', 
                                                                                verticalAlign: 'top',
                                                                                color: '#000000',
                                                                                fontFamily: '"HarmonyOS Sans SC"',
                                                                                fontSize: '14px',
                                                                                fontStyle: 'normal',
                                                                                fontWeight: '400',
                                                                                lineHeight: 'normal',
                                                                                letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                                            }}>
                                                                                Version:
                                                                            </td>
                                                                            <td style={{ 
                                                                                verticalAlign: 'top',
                                                                                color: '#002bb7c4',
                                                                                fontFamily: '"HarmonyOS Sans SC"',
                                                                                fontSize: '14px',
                                                                                fontStyle: 'normal',
                                                                                fontWeight: '400',
                                                                                lineHeight: 'normal',
                                                                                letterSpacing: 'var(--Typography-Letter-spacing-3, 0)'
                                                                            }}>
                                                                                {dependency.version}
                                                                            </td>
                                                                        </tr>
                                                                        <tr>
                                                                            <td style={{ 
                                                                                width: '120px', 
                                                                                verticalAlign: 'top',
                                                                                color: '#000000',
                                                                                fontFamily: '"HarmonyOS Sans SC"',
                                                                                fontSize: '14px',
                                                                                fontStyle: 'normal',
                                                                                fontWeight: '400',
                                                                                lineHeight: 'normal',
                                                                                letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                                            }}>
                                                                                Published:
                                                                            </td>
                                                                            <td style={{ 
                                                                                verticalAlign: 'top',
                                                                                alignSelf: 'stretch',
                                                                                color: '#80838d',
                                                                                fontFamily: '"HarmonyOS Sans SC"',
                                                                                fontSize: '14px',
                                                                                fontStyle: 'normal',
                                                                                fontWeight: '400',
                                                                                lineHeight: 'normal',
                                                                                letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                                            }}>
                                                                                {dependency.published}
                                                                            </td>
                                                                        </tr>
                                                                        <tr>
                                                                            <td style={{ 
                                                                                width: '120px', 
                                                                                verticalAlign: 'top',
                                                                                color: '#000000',
                                                                                fontFamily: '"HarmonyOS Sans SC"',
                                                                                fontSize: '14px',
                                                                                fontStyle: 'normal',
                                                                                fontWeight: '400',
                                                                                lineHeight: 'normal',
                                                                                letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                                            }}>
                                                                                Description:
                                                                            </td>
                                                                            <td style={{ 
                                                                                verticalAlign: 'top',
                                                                                alignSelf: 'stretch',
                                                                                color: '#80838d',
                                                                                fontFamily: '"HarmonyOS Sans SC"',
                                                                                fontSize: '14px',
                                                                                fontStyle: 'normal',
                                                                                fontWeight: '400',
                                                                                lineHeight: 'normal',
                                                                                letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                                            }}>
                                                                                {dependency.description}
                                                                            </td>
                                                                        </tr>
                                                                    </tbody>
                                                                </table>
                                                            </div>
                                                        </td>
                                                    </tr>
                                                )}
                                            </React.Fragment>
                                        ))}
                                    </tbody>
                                </table>
                                </div>
                            )}
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
DependenciesPage.getProviders = (page: any, pageProps: any) => {
    return (
        <AuthAppProviders {...pageProps}>
            <AppLayout {...pageProps}>{page}</AppLayout>
        </AuthAppProviders>
    );
};

export default DependenciesPage;
