"use client";
import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { useRouter } from 'next/router';
import { useParams } from 'next/navigation';
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline';
import { VersionSelectorDropdown } from '../../../../../components/Rust/VersionSelector/VersionSelectorDropdown';

interface CrateInfoLayoutProps {
    children: React.ReactNode;
}

const CrateInfoLayoutComponent = ({ children }: CrateInfoLayoutProps) => {
    const router = useRouter();
    const params = useParams();
    
    // 从查询参数或URL参数中获取crate信息
    const crateName = useMemo(() => 
        (router.query.crateName as string) || params?.crateName as string || "example-crate", 
        [router.query.crateName, params?.crateName]
    );
    const version = useMemo(() => 
        (router.query.version as string) || params?.version as string || "1.0.0", 
        [router.query.version, params?.version]
    );
    const nsfront = useMemo(() => 
        params?.nsfront as string || router.query.org as string, 
        [params?.nsfront, router.query.org]
    );
    
    // 版本选择相关状态
    const [isVersionDialogOpen, setIsVersionDialogOpen] = useState(false);
    const [selectedVersion, setSelectedVersion] = useState<string>(version);
    const [versions] = useState<string[]>(["1.0.0", "1.1.0", "1.2.0", "2.0.0", "0.2.01", "0.2.02", "0.1.06", "0.1.05"]);
    
    // 根据当前路径确定activeTab
    const [activeTab, setActiveTab] = useState<'overview' | 'dependencies' | 'dependents' | 'compare' | 'versions'>('overview');
    
    useEffect(() => {
        const path = router.asPath;
        
        if (path.includes('/dependencies')) {
            setActiveTab('dependencies');
        } else if (path.includes('/dependents')) {
            setActiveTab('dependents');
        } else if (path.includes('/compare')) {
            setActiveTab('compare');
        } else if (path.includes('/versions')) {
            setActiveTab('versions');
        } else {
            setActiveTab('overview');
        }
    }, [router.asPath]);

    const handleTabClick = useCallback((href: string) => {
        router.push(href, undefined, { shallow: true });
    }, [router]);

    // 版本选择处理函数
    const handleVersionSelect = useCallback((newVersion: string) => {
        setSelectedVersion(newVersion);
        // 更新URL中的版本参数
        const currentPath = router.asPath;

        const newPath = currentPath.replace(/[?&]version=[^&]*/, '') + 
            (currentPath.includes('?') ? '&' : '?') + `version=${newVersion}`;
            
        router.push(newPath, undefined, { shallow: true });
    }, [router]);

    const navigationTabs = useMemo(() => [
        { id: 'overview', label: 'overview', href: `/${nsfront}/rust/rust-ecosystem/crate-info?crateName=${crateName}&version=${version}&nsfront=${nsfront}&nsbehind=${router.query.nsbehind || 'rust/rust-ecosystem/crate-info'}` },
        { id: 'dependencies', label: 'dependencies', href: `/${nsfront}/rust/rust-ecosystem/crate-info/dependencies?crateName=${crateName}&version=${version}&nsfront=${nsfront}&nsbehind=${router.query.nsbehind || 'rust/rust-ecosystem/crate-info'}` },
        { id: 'dependents', label: 'dependents', href: `/${nsfront}/rust/rust-ecosystem/crate-info/dependents?crateName=${crateName}&version=${version}&nsfront=${nsfront}&nsbehind=${router.query.nsbehind || 'rust/rust-ecosystem/crate-info'}` },
        { id: 'compare', label: 'compare', href: `/${nsfront}/rust/rust-ecosystem/crate-info/compare?crateName=${crateName}&version=${version}&nsfront=${nsfront}&nsbehind=${router.query.nsbehind || 'rust/rust-ecosystem/crate-info'}` },
        { id: 'versions', label: 'versions', href: `/${nsfront}/rust/rust-ecosystem/crate-info/versions?crateName=${crateName}&version=${version}&nsfront=${nsfront}&nsbehind=${router.query.nsbehind || 'rust/rust-ecosystem/crate-info'}` }
    ], [nsfront, crateName, version, router.query.nsbehind]);

    return (
        <div className="h-screen bg-[#F4F4F5] flex flex-col">
            {/* 搜索栏 - 固定在顶部 */}
            <div className="w-full flex justify-center flex-shrink-0" style={{ background: '#FFF' }}>
                <div
                    className="flex items-center"
                    style={{
                        width: '1680px',
                        height: '53px',
                        flexShrink: 0,
                        marginTop: 0,
                        marginBottom: 0,
                        paddingLeft: 32,
                        paddingRight: 32,
                        background: '#FFF',
                        boxSizing: 'border-box',
                    }}
                >
                    <form className="flex-1 max-w-xl ml-8">
                        <div className="relative ml-10 mt-2">
                            <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                                <MagnifyingGlassIcon className="h-5 w-5 text-gray-400" />
                            </div>
                            <input
                                type="text"
                                placeholder="Search..."
                                className="block w-full pl-10 pr-3 py-2 border-0 focus:ring-0 focus:outline-none bg-transparent text-gray-900 placeholder-gray-500"
                            />
                        </div>
                    </form>
                </div>
                <div 
                    style={{
                        width: '1680px',
                        height: '0px',
                        background: '#F4F4F5',
                        marginTop: 0,
                        marginBottom: 0,
                        paddingLeft: 32,
                        paddingRight: 32,
                        boxSizing: 'border-box',
                    }}
                />
            </div>

            {/* 分类标签和版本选择区域 - 固定在搜索栏下方 */}
            <div className="w-full flex justify-center flex-shrink-0" style={{ background: '#FFF' }}>
                <div style={{ width: '1370px', paddingLeft: 32, paddingRight: 32, paddingTop: 24 }}>
                    {/* Crate信息 */}
                    <div className="flex items-center justify-between mb-6">
                        <div className="flex items-center space-x-4">
                            <div className="flex flex-col space-y-2">
                                <div className="text-sm text-gray-500">Cargo crate</div>
                                <h1 
                                    className="text-3xl font-bold text-gray-900"
                                    style={{
                                        color: '#1c2024',
                                        fontFamily: '"HarmonyOS Sans SC"',
                                        fontSize: '36px',
                                        fontStyle: 'normal',
                                        fontWeight: 400,
                                        lineHeight: 'normal'
                                    }}
                                >
                                    {crateName}
                                </h1>
                            </div>
                            <div className="relative">
                                <button
                                    onClick={() => setIsVersionDialogOpen(!isVersionDialogOpen)}
                                    className="flex items-center space-x-2 mt-8 hover:bg-gray-50 transition-colors"
                                    style={{
                                        display: 'flex',
                                        height: '40px',
                                        padding: '0 16px',
                                        alignItems: 'center',
                                        gap: '12px',
                                        alignSelf: 'stretch',
                                        borderRadius: '6px',
                                        border: '1px solid #00062e33',
                                        background: '#ffffffe6',
                                        cursor: 'pointer'
                                    }}
                                >
                                    <div className="w-6 h-6 border-2 border-gray-400 rounded-full flex items-center justify-center bg-transparent">
                                        <svg className="w-3 h-3 text-gray-400" fill="currentColor" viewBox="0 0 20 20">
                                            <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                                        </svg>
                                    </div>
                                    <span className="text-lg font-medium text-gray-900">{selectedVersion}</span>
                                    <svg className="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                                    </svg>
                                </button>
                                
                                <VersionSelectorDropdown
                                    isOpen={isVersionDialogOpen}
                                    onClose={() => setIsVersionDialogOpen(false)}
                                    onVersionSelect={handleVersionSelect}
                                    currentVersion={selectedVersion}
                                    versions={versions}
                                />
                            </div>
                        </div>
                    </div>

                    {/* 导航标签 */}
                    <div className="flex space-x-8 mb-0">
                        {navigationTabs.map((tab) => (
                            tab.href ? (
                                <button
                                    key={tab.id}
                                    onClick={() => handleTabClick(tab.href!)}
                                    className={`py-2 px-1 border-b-2 transition-colors ${
                                        tab.id === activeTab
                                            ? 'border-blue-500'
                                            : 'border-transparent hover:text-gray-700 hover:border-gray-300'
                                    }`}
                                    style={{
                                        color: tab.id === activeTab ? '#1c2024' : '#6b7280',
                                        fontFamily: '"HarmonyOS Sans SC"',
                                        fontSize: '16px',
                                        fontStyle: 'normal',
                                        fontWeight: 500,
                                        lineHeight: '20px',
                                        letterSpacing: '0',
                                    }}
                                >
                                    {tab.label}
                                </button>
                            ) : (
                                <button
                                    key={tab.id}
                                    className={`py-2 px-1 border-b-2 transition-colors ${
                                        tab.id === activeTab
                                            ? 'border-blue-500'
                                            : 'border-transparent hover:text-gray-700 hover:border-gray-300'
                                    }`}
                                    style={{
                                        color: tab.id === activeTab ? '#1c2024' : '#6b7280',
                                        fontFamily: '"HarmonyOS Sans SC"',
                                        fontSize: '16px',
                                        fontStyle: 'normal',
                                        fontWeight: 500,
                                        lineHeight: '20px',
                                        letterSpacing: '0',
                                    }}
                                >
                                    {tab.label}
                                </button>
                            )
                        ))}
                    </div>
                </div>
            </div>

            {/* 可滚动内容区域 */}
            <div className="flex-1 overflow-auto" style={{ background: '#F4F4F5' }}>
                {children}
            </div>
        </div>
    );
};

const CrateInfoLayout = React.memo(CrateInfoLayoutComponent);

CrateInfoLayout.displayName = 'CrateInfoLayout';

export default CrateInfoLayout; 