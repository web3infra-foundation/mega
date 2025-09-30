"use client";
import React, { useEffect, useState, useRef, useCallback } from 'react';
import Head from 'next/head';
import Image from 'next/image';
import { useParams } from 'next/navigation';
import { useRouter } from 'next/router';
import { AppLayout } from '@/components/Layout/AppLayout';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
// import { ArrowsRightLeftIcon, ChevronDownIcon } from '@heroicons/react/24/outline';
import CrateInfoLayout from '../layout';

// API 响应接口
interface CrateInfo {
    crate_name: string;
    description: string;
    dependencies: {
        direct: number;
        indirect: number;
    };
    dependents: {
        direct: number;
        indirect: number;
    };
    cves: Array<{
        id: string;
        subtitle: string;
        reported: string;
        issued: string;
        package: string;
        ttype: string;
        keywords: string;
        aliases: string;
        reference: string;
        patched: string;
        unaffected: string;
        description: string;
        url: string;
    }>;
    dep_cves: Array<{
        id: string;
        subtitle: string;
        reported: string;
        issued: string;
        package: string;
        ttype: string;
        keywords: string;
        aliases: string;
        reference: string;
        patched: string;
        unaffected: string;
        description: string;
        url: string;
    }>;
    license: string;
    github_url: string;
    doc_url: string;
    versions: string[];
}

// 比较用的版本数据接口
interface VersionData {
    version: string;
    published: string;
    description: string;
    license: string;
    github_url: string;
    doc_url: string;
    dependencies: {
        direct: number;
        indirect: number;
    };
    dependents: {
        direct: number;
        indirect: number;
    };
    cves: Array<{
        id: string;
        subtitle: string;
        description: string;
        highlighted?: 'added' | 'removed' | 'updated';
    }>;
    dep_cves: Array<{
        id: string;
        subtitle: string;
        description: string;
        highlighted?: 'added' | 'removed' | 'updated';
    }>;
}

const ComparePage = () => {
    const params = useParams();
    const router = useRouter();
    const [leftVersion, setLeftVersion] = useState<VersionData | null>(null);
    const [rightVersion, setRightVersion] = useState<VersionData | null>(null);
    const [selectedLeftVersion, setSelectedLeftVersion] = useState('');
    const [selectedRightVersion, setSelectedRightVersion] = useState('');
    const [isSwapped, setIsSwapped] = useState(false);
    const [isLeftVersionDialogOpen, setIsLeftVersionDialogOpen] = useState(false);
    const [isRightVersionDialogOpen, setIsRightVersionDialogOpen] = useState(false);
    const [versions, setVersions] = useState<string[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // 从URL参数中获取crate信息
    const crateName = params?.name as string || "tokio";
    const version = params?.version as string || "1.2.01";
    const nsfront = params?.nsfront as string || router.query.org as string;
    const nsbehind = params?.nsbehind as string || "rust/rust-ecosystem/crate-info";

    // 从 API 获取版本数据
    const fetchVersionData = useCallback(async (version: string): Promise<VersionData | null> => {
        try {
            const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL;
            const response = await fetch(`${apiBaseUrl}/api/crates/${nsfront}/${nsbehind}/${crateName}/${version}`);
            
            if (!response.ok) {
                throw new Error(`Failed to fetch data for version ${version}`);
            }
            
            const data: CrateInfo = await response.json();
            
            // 转换 API 数据为比较用的格式
            return {
                version: version,
                published: 'Unknown', // API 中没有发布时间，使用默认值
                description: data.description || 'No description available',
                license: data.license || 'Unknown',
                github_url: data.github_url || '',
                doc_url: data.doc_url || '',
                dependencies: data.dependencies,
                dependents: data.dependents,
                cves: data.cves.map(cve => ({
                    id: cve.id,
                    subtitle: cve.subtitle || cve.description,
                    description: cve.description,
                    highlighted: undefined // 将在比较时设置
                })),
                dep_cves: data.dep_cves.map(cve => ({
                    id: cve.id,
                    subtitle: cve.subtitle || cve.description,
                    description: cve.description,
                    highlighted: undefined // 将在比较时设置
                }))
            };
        } catch (err) {
            return null;
        }
    }, [nsfront, nsbehind, crateName]);

    // 比较两个版本的数据并标记差异
    const compareVersions = (left: VersionData, right: VersionData) => {
        const leftWithHighlights = { ...left };
        const rightWithHighlights = { ...right };

        // 比较 CVE 数据 (In the Dependencies 部分)
        const leftCveIds = new Set(left.cves.map(cve => cve.id));
        const rightCveIds = new Set(right.cves.map(cve => cve.id));

        leftWithHighlights.cves = left.cves.map(cve => ({
            ...cve,
            highlighted: rightCveIds.has(cve.id) ? undefined : 'removed'
        }));

        rightWithHighlights.cves = right.cves.map(cve => ({
            ...cve,
            highlighted: leftCveIds.has(cve.id) ? undefined : 'added'
        }));

        // 比较依赖 CVE 数据 (Dependencies 部分)
        const leftDepCveIds = new Set(left.dep_cves.map(cve => cve.id));
        const rightDepCveIds = new Set(right.dep_cves.map(cve => cve.id));

        leftWithHighlights.dep_cves = left.dep_cves.map(cve => ({
            ...cve,
            highlighted: rightDepCveIds.has(cve.id) ? undefined : 'removed'
        }));

        rightWithHighlights.dep_cves = right.dep_cves.map(cve => ({
            ...cve,
            highlighted: leftDepCveIds.has(cve.id) ? undefined : 'added'
        }));

        return { leftWithHighlights, rightWithHighlights };
    };

    // 获取版本列表并设置默认版本
    useEffect(() => {
        const fetchVersions = async () => {
            try {
                const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL;
                const response = await fetch(`${apiBaseUrl}/api/crates/${nsfront}/${nsbehind}/${crateName}/${version}`);
                
                if (!response.ok) {
                    throw new Error('Failed to fetch versions');
                }
                
                const data: CrateInfo = await response.json();

                const versionList = data.versions || [];

                setVersions(versionList);
                
                // 设置默认版本：右边是最新版本（索引0），左边是次新版本（索引1）
                if (versionList.length >= 2) {
                    setSelectedRightVersion(versionList[0]); // 最新版本
                    setSelectedLeftVersion(versionList[1]);  // 次新版本
                } else if (versionList.length === 1) {
                    // 如果只有一个版本，左右都设置为同一个版本
                    setSelectedRightVersion(versionList[0]);
                    setSelectedLeftVersion(versionList[0]);
                }
            } catch (err) {
                setError('Failed to load versions');
            }
        };

        if (crateName && version && nsfront && nsbehind) {
            fetchVersions();
        }
    }, [crateName, version, nsfront, nsbehind]);

    // 获取和比较版本数据
    useEffect(() => {
        const fetchAndCompareVersions = async () => {
            if (!selectedLeftVersion || !selectedRightVersion) return;
            
            setLoading(true);
            setError(null);
            
            try {
                const [leftData, rightData] = await Promise.all([
                    fetchVersionData(selectedLeftVersion),
                    fetchVersionData(selectedRightVersion)
                ]);

                if (leftData && rightData) {
                    const { leftWithHighlights, rightWithHighlights } = compareVersions(leftData, rightData);

                    setLeftVersion(leftWithHighlights);
                    setRightVersion(rightWithHighlights);
                } else {
                    setError('Failed to load version data');
                }
            } catch (err) {
                setError('Failed to compare versions');
            } finally {
                setLoading(false);
            }
        };

        fetchAndCompareVersions();
    }, [selectedLeftVersion, selectedRightVersion, fetchVersionData]);

    const handleSwapVersions = () => {
        setIsSwapped(!isSwapped);
    };

    const handleLeftVersionSelect = (version: string) => {
        setSelectedLeftVersion(version);
        setIsLeftVersionDialogOpen(false);
    };

    const handleRightVersionSelect = (version: string) => {
        setSelectedRightVersion(version);
        setIsRightVersionDialogOpen(false);
    };

    // 版本选择下拉框组件
    const VersionSelectorDropdown = ({ 
        isOpen, 
        onClose, 
        onVersionSelect, 
        currentVersion, 
        versions 
    }: { 
        isOpen: boolean; 
        onClose: () => void; 
        onVersionSelect: (version: string) => void; 
        currentVersion: string; 
        versions: string[]; 
    }) => {
        const dropdownRef = useRef<HTMLDivElement>(null);

        const handleVersionSelect = (version: string) => {
            onVersionSelect(version);
            onClose();
        };

        useEffect(() => {
            const handleClickOutside = (event: MouseEvent) => {
                if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
                    onClose();
                }
            };

            if (isOpen) {
                document.addEventListener('mousedown', handleClickOutside);
            }

            return () => {
                document.removeEventListener('mousedown', handleClickOutside);
            };
        }, [isOpen, onClose]);

        if (!isOpen) return null;

        return (
            <div 
                ref={dropdownRef}
                className="absolute top-full left-0 mt-1 w-full bg-white border border-gray-300 rounded-md shadow-lg z-50"
                style={{ width: '140px' }}
            >
                <div className="p-3">
                    <div className="max-h-48 overflow-y-auto">
                        <div className="mb-0">
                            <div className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2 pl-0">
                                Default
                            </div>
                            <div className="space-y-1">
                                <button
                                    onClick={() => handleVersionSelect(currentVersion)}
                                    className="w-full text-left px-0 py-1 rounded hover:bg-gray-100"
                                >
                                    <span className="text-sm text-gray-900">{currentVersion}</span>
                                </button>
                            </div>
                        </div>

                        <div>
                            <div 
                                style={{
                                    display: 'flex',
                                    padding: 'var(--Spacing-2, 8px) var(--Spacing-3, 12px)',
                                    alignItems: 'center',
                                    alignSelf: 'stretch',
                                    background: '#ffffff00',
                                    marginTop: '1px',
                                    marginBottom: '6px'
                                }}
                            >
                                <div className="w-full bg-gray-200" style={{ marginLeft: '-12px', marginRight: '-2px', height: '1.5px' }}></div>
                            </div>
                            <div className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2 pl-0">
                                ALL
                            </div>
                            <div className="space-y-1">
                                {versions.map((version: string) => (
                                    <button
                                        key={version}
                                        onClick={() => handleVersionSelect(version)}
                                        className="w-full text-left px-0 py-1 rounded hover:bg-gray-100"
                                    >
                                        <span className="text-sm text-gray-900">{version}</span>
                                    </button>
                                ))}
                            </div>
                        </div>
                    </div>

                    <div className="border-t pt-3 mt-3">
                        <button
                            onClick={() => {
                                // console.log('View all versions');
                            }}
                            style={{
                                flex: '1 0 0',
                                color: '#3a5bc7',
                                fontFamily: '"SF Pro"',
                                fontSize: '14px',
                                fontStyle: 'normal',
                                fontWeight: 400,
                                lineHeight: '20px',
                                letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                            }}
                        >
                            View all versions
                        </button>
                    </div>
                </div>
            </div>
        );
    };

    const VersionSelector = ({ 
        version,
        isOpen,
        onToggle,
        onVersionSelect,
        versions
    }: { 
        version: string;
        isOpen: boolean;
        onToggle: () => void;
        onVersionSelect: (version: string) => void;
        versions: string[];
    }) => (
        <div className="relative">
            <button
                onClick={onToggle}
                className="flex items-center space-x-2 hover:bg-gray-50 transition-colors"
                style={{
                    display: 'flex',
                    width: '140px',
                    height: '40px',
                    padding: '0 12px',
                    alignItems: 'center',
                    gap: '10px',
                    borderRadius: '6px',
                    border: '1px solid #00062e33',
                    background: '#ffffffe6',
                    cursor: 'pointer'
                }}
            >
                <div 
                    className="flex items-center justify-center bg-transparent"
                    style={{
                        width: '24px',
                        height: '24px',
                        border: '2px solid #9ca3af',
                        borderRadius: '50%',
                        flexShrink: 0
                    }}
                >
                    <svg 
                        className="text-gray-400" 
                        fill="currentColor" 
                        viewBox="0 0 20 20"
                        style={{
                            width: '12px',
                            height: '12px'
                        }}
                    >
                        <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                    </svg>
                </div>
                <span className="text-lg font-medium text-gray-900" style={{
                    fontFamily: '"HarmonyOS Sans SC"',
                    fontSize: '16px',
                    fontWeight: 500,
                    color: '#1c2024'
                }}>
                    {version}
                </span>
                <svg className="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
            </button>
            
            <VersionSelectorDropdown
                isOpen={isOpen}
                onClose={() => onToggle()}
                onVersionSelect={onVersionSelect}
                currentVersion={version}
                versions={versions}
            />
        </div>
    );

    // const ComparisonSection = ({ 
    //     title, 
    //     leftContent, 
    //     rightContent 
    // }: { 
    //     title: string; 
    //     leftContent: React.ReactNode; 
    //     rightContent: React.ReactNode; 
    // }) => (
    //     <div className="mb-8">
    //         <h3 
    //             className="text-lg font-semibold mb-4"
    //             style={{
    //                 color: '#333333',
    //                 fontFamily: '"HarmonyOS Sans SC"',
    //                 fontSize: '18px',
    //                 fontWeight: 600
    //             }}
    //         >
    //             {title}
    //         </h3>
    //         <div className="grid grid-cols-2 gap-8">
    //             <div>{leftContent}</div>
    //             <div>{rightContent}</div>
    //         </div>
    //     </div>
    // );

    const getHighlightColor = (type?: 'added' | 'removed' | 'updated') => {
        switch (type) {
            case 'added': return '#65BA74'; // 绿色 - 新增
            case 'removed': return '#E5484D'; // 红色 - 移除
            case 'updated': return '#F59E0B'; // 橙色 - 更新
            default: return 'transparent';
        }
    };

    if (loading) {
        return (
            <div className="flex justify-center items-center min-h-screen">
                <div className="text-gray-500">Loading comparison data...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="flex justify-center items-center min-h-screen">
                <div className="text-red-500">Error: {error}</div>
            </div>
        );
    }

    if (!selectedLeftVersion || !selectedRightVersion) {
        return (
            <div className="flex justify-center items-center min-h-screen">
                <div className="text-gray-500">Loading versions...</div>
            </div>
        );
    }

    if (!leftVersion || !rightVersion) {
        return (
            <div className="flex justify-center items-center min-h-screen">
                <div className="text-gray-500">No version data available</div>
            </div>
        );
    }

    return (
        <>
            <Head>
                <title>Compare - {crateName}</title>
            </Head>
            <CrateInfoLayout>
                <div className="flex justify-center">
                    <div className="w-[1370px] px-8 py-4" style={{ paddingLeft: '32px', paddingRight: '32px' }}>
                        {/* 左右两个卡片 */}
                        <div className="flex justify-start gap-24 relative">
                                                         {/* 交换按钮 - 位于顶部中间 */}
                             <div className="absolute top-0 left-1/2 transform -translate-x-1/2 -translate-y-1/2 z-10" style={{ marginLeft: '-16px', marginTop: '20px' }}>
                                 <button
                                     onClick={handleSwapVersions}
                                     title="交换版本"
                                     style={{
                                         background: 'transparent',
                                         border: 'none',
                                         padding: 0,
                                         cursor: 'pointer'
                                     }}
                                 >
                                     <Image 
                                         src="/rust/rust-ecosystem/crate-info/compare/exchange.png" 
                                         alt="exchange" 
                                         width={32}
                                         height={32}
                                     />
                                 </button>
                             </div>
                            
                            {/* 左侧卡片 */}
                            <div 
                                style={{
                                    width: '590px',
                                    flexShrink: 0,
                                    borderRadius: 'var(--Radius-6, 16px)',
                                    border: '1px solid var(--Colors-Neutral-Neutral-Alpha-6, #00002f26)',
                                    background: 'var(--Panel-default, #ffffffcc)',
                                    padding: '20px',
                                    order: isSwapped ? 2 : 1
                                }}
                            >
                                {/* 版本选择器 */}
                                <div className="mb-6">
                                    <VersionSelector 
                                        version={selectedLeftVersion}
                                        isOpen={isLeftVersionDialogOpen}
                                        onToggle={() => setIsLeftVersionDialogOpen(!isLeftVersionDialogOpen)}
                                        onVersionSelect={handleLeftVersionSelect}
                                        versions={versions}
                                    />
                                </div>
                                {/* Published */}
                                <div className="mb-8">
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '16px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Published
                                    </h3>
                                    <div 
                                        className="p-3 rounded"
                                        style={{ 
                                            width: '590px',
                                            height: '24px',
                                            flexShrink: 0,
                                            background: 'transparent',
                                            color: '#333333',
                                            display: 'flex',
                                            alignItems: 'center',
                                            padding: '0 12px',
                                            marginLeft: '-12px',
                                            marginRight: '-20px',
                                            borderRadius: '0'
                                        }}
                                    >
                                        <div style={{ 
                                            alignSelf: 'stretch',
                                            color: '#333333',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '16px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}>
                                            {leftVersion.published}
                                        </div>
                                    </div>
                                </div>

                                {/* Description */}
                                <div className="mb-8">
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '16px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Description
                                    </h3>
                                    <div style={{ 
                                        alignSelf: 'stretch',
                                        color: '#00071b80',
                                        fontFamily: '"HarmonyOS Sans SC"',
                                        fontSize: '16px',
                                        fontStyle: 'normal',
                                        fontWeight: 400,
                                        lineHeight: '20px'
                                    }}>
                                        {leftVersion.description}
                                    </div>
                                </div>

                                {/* Licenses */}
                                <div className="mb-8">
                                    {/* 分割线 */}
                                    <div 
                                        style={{
                                            width: '510px',
                                            height: '0',
                                            borderTop: '1px solid var(--Colors-Neutral-Neutral-Alpha-6, #00002f26)',
                                            marginBottom: '16px'
                                        }}
                                    />
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '24px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Licenses
                                    </h3>
                                    <div className="space-y-3">
                                        <div>
                                            <div 
                                                className="text-xs text-gray-500 uppercase mb-1"
                                                style={{
                                                    alignSelf: 'stretch',
                                                    color: '#000509e3',
                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                    fontSize: '16px',
                                                    fontStyle: 'normal',
                                                    fontWeight: 400,
                                                    lineHeight: '20px'
                                                }}
                                            >
                                                LICENSES
                                            </div>
                                            <div 
                                                className="font-medium"
                                                style={{
                                                    alignSelf: 'stretch',
                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                    fontSize: '16px',
                                                    fontStyle: 'normal',
                                                    fontWeight: 400,
                                                    lineHeight: '20px',
                                                    backgroundColor: leftVersion.license !== rightVersion.license ? getHighlightColor('updated') : 'transparent',
                                                    color: leftVersion.license !== rightVersion.license ? 'white' : '#00071b80',
                                                    padding: leftVersion.license !== rightVersion.license ? '4px 8px' : '0',
                                                    borderRadius: leftVersion.license !== rightVersion.license ? '4px' : '0'
                                                }}
                                            >
                                                {leftVersion.license}
                                            </div>
                                        </div>
                                        <div>
                                            <div 
                                                className="text-xs text-gray-500 uppercase mb-2"
                                                style={{
                                                    alignSelf: 'stretch',
                                                    color: '#000509e3',
                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                    fontSize: '16px',
                                                    fontStyle: 'normal',
                                                    fontWeight: 400,
                                                    lineHeight: '20px'
                                                }}
                                            >
                                                DEPENDENCY LICENSES
                                            </div>
                                            <div className="space-y-1">
                                                <div 
                                                    className="p-1 text-sm"
                                                    style={{ 
                                                        fontFamily: '"HarmonyOS Sans SC"',
                                                        fontSize: '16px',
                                                        fontStyle: 'normal',
                                                        fontWeight: 400,
                                                        lineHeight: '20px',
                                                        color: '#00071b80'
                                                    }}
                                                >
                                                    Dependencies: {leftVersion.dependencies.direct + leftVersion.dependencies.indirect} total
                                                    <br />
                                                    Direct: {leftVersion.dependencies.direct}, Indirect: {leftVersion.dependencies.indirect}
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>

                                {/* Security Advisories */}
                                <div className="mb-8">
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '24px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Security Advisories
                                    </h3>
                                    <div>
                                        <div 
                                            className="text-xs text-gray-500 uppercase mb-2"
                                            style={{
                                                alignSelf: 'stretch',
                                                color: '#000509e3',
                                                fontFamily: '"HarmonyOS Sans SC"',
                                                fontSize: '16px',
                                                fontStyle: 'normal',
                                                fontWeight: 400,
                                                lineHeight: '20px'
                                            }}
                                        >
                                            IN THE DEPENDENCIES
                                        </div>
                                        <div className="space-y-2">
                                            {leftVersion.cves.length > 0 ? (
                                                leftVersion.cves.map((cve) => (
                                                    <div 
                                                        key={`left-cve-${cve.id}`}
                                                        className="p-2 rounded"
                                                        style={{ 
                                                            backgroundColor: cve.highlighted ? getHighlightColor(cve.highlighted) : 'transparent',
                                                            color: cve.highlighted ? 'white' : '#333333',
                                                            display: 'flex',
                                                            alignItems: 'center',
                                                            justifyContent: 'space-between',
                                                            padding: cve.highlighted ? '8px 12px' : '8px',
                                                            borderRadius: cve.highlighted ? '4px' : '0'
                                                        }}
                                                    >
                                                        <div 
                                                            style={{
                                                                display: 'flex',
                                                                alignItems: 'center',
                                                                justifyContent: 'space-between',
                                                                width: '100%'
                                                            }}
                                                        >
                                                            <div 
                                                                className="font-medium text-sm"
                                                                style={{
                                                                    color: cve.highlighted ? 'white' : '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px'
                                                                }}
                                                            >
                                                                {cve.id}
                                                            </div>
                                                            <div 
                                                                className="text-sm"
                                                                style={{ 
                                                                    color: cve.highlighted ? 'white' : '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px',
                                                                    marginLeft: '16px'
                                                                }}
                                                            >
                                                                {cve.subtitle}
                                                            </div>
                                                        </div>
                                                        {cve.highlighted && (
                                                            <svg 
                                                                fill="currentColor" 
                                                                viewBox="0 0 20 20"
                                                                style={{
                                                                    width: '16px',
                                                                    height: '16px',
                                                                    color: 'white',
                                                                    marginLeft: '8px'
                                                                }}
                                                            >
                                                                <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                                                            </svg>
                                                        )}
                                                    </div>
                                                ))
                                            ) : (
                                                <div 
                                                    className="p-2 text-sm"
                                                    style={{ 
                                                        color: '#00071b80',
                                                        fontFamily: '"HarmonyOS Sans SC"',
                                                        fontSize: '14px',
                                                        fontStyle: 'normal',
                                                        fontWeight: 400,
                                                        lineHeight: '20px'
                                                    }}
                                                >
                                                    No security advisories in dependencies
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                </div>

                                {/* Dependencies */}
                                <div className="mb-8">
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '24px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Dependencies
                                    </h3>
                                                                         <div className="space-y-2">
                                            {leftVersion.dep_cves.length > 0 ? (
                                                leftVersion.dep_cves.map((cve) => (
                                                    <div 
                                                        key={`left-dep-cve-${cve.id}`}
                                                        className="p-2 rounded"
                                                        style={{ 
                                                            backgroundColor: cve.highlighted ? getHighlightColor(cve.highlighted) : 'transparent',
                                                            color: cve.highlighted ? 'white' : '#333333',
                                                            display: 'flex',
                                                            alignItems: 'center',
                                                            justifyContent: 'space-between',
                                                            padding: cve.highlighted ? '8px 12px' : '8px',
                                                            borderRadius: cve.highlighted ? '4px' : '0'
                                                        }}
                                                    >
                                                        <div 
                                                            style={{
                                                                display: 'flex',
                                                                alignItems: 'center',
                                                                justifyContent: 'space-between',
                                                                width: '100%'
                                                            }}
                                                        >
                                                            <div 
                                                                className="font-medium text-sm"
                                                                style={{
                                                                    color: cve.highlighted ? 'white' : '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px'
                                                                }}
                                                            >
                                                                {cve.id}
                                                            </div>
                                                            <div 
                                                                className="text-sm"
                                                                style={{ 
                                                                    color: cve.highlighted ? 'white' : '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px',
                                                                    marginLeft: '16px'
                                                                }}
                                                            >
                                                                {cve.subtitle}
                                                            </div>
                                                        </div>
                                                        {cve.highlighted && (
                                                            <svg 
                                                                fill="currentColor" 
                                                                viewBox="0 0 20 20"
                                                                style={{
                                                                    width: '16px',
                                                                    height: '16px',
                                                                    color: 'white',
                                                                    marginLeft: '8px'
                                                                }}
                                                            >
                                                                <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                                                            </svg>
                                                        )}
                                                    </div>
                                                ))
                                            ) : (
                                                <div 
                                                    className="p-2 text-sm"
                                                    style={{ 
                                                        color: '#00071b80',
                                                        fontFamily: '"HarmonyOS Sans SC"',
                                                        fontSize: '14px',
                                                        fontStyle: 'normal',
                                                        fontWeight: 400,
                                                        lineHeight: '20px'
                                                    }}
                                                >
                                                    No dependency security advisories
                                                </div>
                                            )}
                                        </div>
                                </div>
                            </div>

                            {/* 右侧卡片 */}
                            <div 
                                style={{
                                    width: '590px',
                                    flexShrink: 0,
                                    borderRadius: 'var(--Radius-6, 16px)',
                                    border: '1px solid var(--Colors-Neutral-Neutral-Alpha-6, #00002f26)',
                                    background: 'var(--Panel-default, #ffffffcc)',
                                    padding: '20px',
                                    order: isSwapped ? 1 : 2
                                }}
                            >
                                {/* 版本选择器 */}
                                <div className="mb-6">
                                    <VersionSelector 
                                        version={selectedRightVersion}
                                        isOpen={isRightVersionDialogOpen}
                                        onToggle={() => setIsRightVersionDialogOpen(!isRightVersionDialogOpen)}
                                        onVersionSelect={handleRightVersionSelect}
                                        versions={versions}
                                    />
                                </div>
                                {/* Published */}
                                <div className="mb-8">
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '16px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Published
                                    </h3>
                                                                                 <div
                 className="p-3 rounded"
                 style={{
                     width: '590px',
                     height: '24px',
                     flexShrink: 0,
                     background: 'var(--Colors-Grass-8, #65BA74)',
                     color: 'white',
                     display: 'flex',
                     alignItems: 'center',
                     justifyContent: 'space-between',
                     padding: '0 12px',
                     marginLeft: '-20px',
                     marginRight: '-20px',
                     borderRadius: '0'
                 }}
             >
                                         <div style={{ 
                                             alignSelf: 'stretch',
                                             color: '#ffffff',
                                             fontFamily: '"HarmonyOS Sans SC"',
                                             fontSize: '16px',
                                             fontStyle: 'normal',
                                             fontWeight: 400,
                                             lineHeight: '20px',
                                             marginLeft: '9px'
                                         }}>
                                             {rightVersion.published}
                                         </div>
                                         <svg 
                                             fill="currentColor" 
                                             viewBox="0 0 20 20"
                                             style={{
                                                 width: '16px',
                                                 height: '16px',
                                                 color: 'white'
                                             }}
                                         >
                                             <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                                         </svg>
                                     </div>
                                </div>

                                {/* Description */}
                                <div className="mb-8">
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '16px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Description
                                    </h3>
                                    <div style={{ 
                                        alignSelf: 'stretch',
                                        color: '#00071b80',
                                        fontFamily: '"HarmonyOS Sans SC"',
                                        fontSize: '16px',
                                        fontStyle: 'normal',
                                        fontWeight: 400,
                                        lineHeight: '20px'
                                    }}>
                                        {rightVersion.description}
                                    </div>
                                </div>

                                {/* Licenses */}
                                <div className="mb-8">
                                    {/* 分割线 */}
                                    <div 
                                        style={{
                                            width: '510px',
                                            height: '0',
                                            borderTop: '1px solid var(--Colors-Neutral-Neutral-Alpha-6, #00002f26)',
                                            marginBottom: '16px'
                                        }}
                                    />
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '24px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Licenses
                                    </h3>
                                    <div className="space-y-3">
                                        <div>
                                            <div 
                                                className="text-xs text-gray-500 uppercase mb-1"
                                                style={{
                                                    alignSelf: 'stretch',
                                                    color: '#000509e3',
                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                    fontSize: '16px',
                                                    fontStyle: 'normal',
                                                    fontWeight: 400,
                                                    lineHeight: '20px'
                                                }}
                                            >
                                                LICENSES
                                            </div>
                                            <div 
                                                className="font-medium"
                                                style={{
                                                    alignSelf: 'stretch',
                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                    fontSize: '16px',
                                                    fontStyle: 'normal',
                                                    fontWeight: 400,
                                                    lineHeight: '20px',
                                                    backgroundColor: leftVersion.license !== rightVersion.license ? getHighlightColor('updated') : 'transparent',
                                                    color: leftVersion.license !== rightVersion.license ? 'white' : '#00071b80',
                                                    padding: leftVersion.license !== rightVersion.license ? '4px 8px' : '0',
                                                    borderRadius: leftVersion.license !== rightVersion.license ? '4px' : '0'
                                                }}
                                            >
                                                {rightVersion.license}
                                            </div>
                                        </div>
                                        <div>
                                            <div 
                                                className="text-xs text-gray-500 uppercase mb-2"
                                                style={{
                                                    alignSelf: 'stretch',
                                                    color: '#000509e3',
                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                    fontSize: '16px',
                                                    fontStyle: 'normal',
                                                    fontWeight: 400,
                                                    lineHeight: '20px'
                                                }}
                                            >
                                                DEPENDENCY LICENSES
                                            </div>
                                            <div className="space-y-1">
                                                <div 
                                                    className="p-1 text-sm"
                                                    style={{ 
                                                        fontFamily: '"HarmonyOS Sans SC"',
                                                        fontSize: '16px',
                                                        fontStyle: 'normal',
                                                        fontWeight: 400,
                                                        lineHeight: '20px',
                                                        color: '#00071b80'
                                                    }}
                                                >
                                                    Dependencies: {rightVersion.dependencies.direct + rightVersion.dependencies.indirect} total
                                                    <br />
                                                    Direct: {rightVersion.dependencies.direct}, Indirect: {rightVersion.dependencies.indirect}
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>

                                {/* Security Advisories */}
                                <div className="mb-8">
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '24px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Security Advisories
                                    </h3>
                                    <div>
                                        <div 
                                            className="text-xs text-gray-500 uppercase mb-2"
                                            style={{
                                                alignSelf: 'stretch',
                                                color: '#000509e3',
                                                fontFamily: '"HarmonyOS Sans SC"',
                                                fontSize: '16px',
                                                fontStyle: 'normal',
                                                fontWeight: 400,
                                                lineHeight: '20px'
                                            }}
                                        >
                                            IN THE DEPENDENCIES
                                        </div>
                                        <div className="space-y-2">
                                            {rightVersion.cves.length > 0 ? (
                                                rightVersion.cves.map((cve) => (
                                                    <div 
                                                        key={`right-cve-${cve.id}`}
                                                        className="p-2 rounded"
                                                        style={{ 
                                                            backgroundColor: cve.highlighted ? getHighlightColor(cve.highlighted) : 'transparent',
                                                            color: cve.highlighted ? 'white' : '#333333',
                                                            display: 'flex',
                                                            alignItems: 'center',
                                                            justifyContent: 'space-between',
                                                            padding: cve.highlighted ? '8px 12px' : '8px',
                                                            borderRadius: cve.highlighted ? '4px' : '0'
                                                        }}
                                                    >
                                                        <div 
                                                            style={{
                                                                display: 'flex',
                                                                alignItems: 'center',
                                                                justifyContent: 'space-between',
                                                                width: '100%'
                                                            }}
                                                        >
                                                            <div 
                                                                className="font-medium text-sm"
                                                                style={{
                                                                    color: cve.highlighted ? 'white' : '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px'
                                                                }}
                                                            >
                                                                {cve.id}
                                                            </div>
                                                            <div 
                                                                className="text-sm"
                                                                style={{ 
                                                                    color: cve.highlighted ? 'white' : '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px',
                                                                    marginLeft: '16px'
                                                                }}
                                                            >
                                                                {cve.subtitle}
                                                            </div>
                                                        </div>
                                                        {cve.highlighted && (
                                                            <svg 
                                                                fill="currentColor" 
                                                                viewBox="0 0 20 20"
                                                                style={{
                                                                    width: '16px',
                                                                    height: '16px',
                                                                    color: 'white',
                                                                    marginLeft: '8px'
                                                                }}
                                                            >
                                                                <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                                                            </svg>
                                                        )}
                                                    </div>
                                                ))
                                            ) : (
                                                <div 
                                                    className="p-2 text-sm"
                                                    style={{ 
                                                        color: '#00071b80',
                                                        fontFamily: '"HarmonyOS Sans SC"',
                                                        fontSize: '14px',
                                                        fontStyle: 'normal',
                                                        fontWeight: 400,
                                                        lineHeight: '20px'
                                                    }}
                                                >
                                                    No security advisories in dependencies
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                </div>

                                {/* Dependencies */}
                                <div className="mb-8">
                                    <h3 
                                        className="text-lg font-semibold mb-4"
                                        style={{
                                            alignSelf: 'stretch',
                                            color: '#000509e3',
                                            fontFamily: '"HarmonyOS Sans SC"',
                                            fontSize: '24px',
                                            fontStyle: 'normal',
                                            fontWeight: 400,
                                            lineHeight: '20px'
                                        }}
                                    >
                                        Dependencies
                                    </h3>
                                                                         <div className="space-y-2">
                                            {rightVersion.dep_cves.length > 0 ? (
                                                rightVersion.dep_cves.map((cve) => (
                                                    <div 
                                                        key={`right-dep-cve-${cve.id}`}
                                                        className="p-2 rounded"
                                                        style={{ 
                                                            backgroundColor: cve.highlighted ? getHighlightColor(cve.highlighted) : 'transparent',
                                                            color: cve.highlighted ? 'white' : '#333333',
                                                            display: 'flex',
                                                            alignItems: 'center',
                                                            justifyContent: 'space-between',
                                                            padding: cve.highlighted ? '8px 12px' : '8px',
                                                            borderRadius: cve.highlighted ? '4px' : '0'
                                                        }}
                                                    >
                                                        <div 
                                                            style={{
                                                                display: 'flex',
                                                                alignItems: 'center',
                                                                justifyContent: 'space-between',
                                                                width: '100%'
                                                            }}
                                                        >
                                                            <div 
                                                                className="font-medium text-sm"
                                                                style={{
                                                                    color: cve.highlighted ? 'white' : '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px'
                                                                }}
                                                            >
                                                                {cve.id}
                                                            </div>
                                                            <div 
                                                                className="text-sm"
                                                                style={{ 
                                                                    color: cve.highlighted ? 'white' : '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px',
                                                                    marginLeft: '16px'
                                                                }}
                                                            >
                                                                {cve.subtitle}
                                                            </div>
                                                        </div>
                                                        {cve.highlighted && (
                                                            <svg 
                                                                fill="currentColor" 
                                                                viewBox="0 0 20 20"
                                                                style={{
                                                                    width: '16px',
                                                                    height: '16px',
                                                                    color: 'white',
                                                                    marginLeft: '8px'
                                                                }}
                                                            >
                                                                <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                                                            </svg>
                                                        )}
                                                    </div>
                                                ))
                                            ) : (
                                                <div 
                                                    className="p-2 text-sm"
                                                    style={{ 
                                                        color: '#00071b80',
                                                        fontFamily: '"HarmonyOS Sans SC"',
                                                        fontSize: '14px',
                                                        fontStyle: 'normal',
                                                        fontWeight: 400,
                                                        lineHeight: '20px'
                                                    }}
                                                >
                                                    No dependency security advisories
                                                </div>
                                            )}
                                        </div>
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
ComparePage.getProviders = (page: any, pageProps: any) => {
    return (
        <AuthAppProviders {...pageProps}>
            <AppLayout {...pageProps}>{page}</AppLayout>
        </AuthAppProviders>
    );
};

export default ComparePage;
