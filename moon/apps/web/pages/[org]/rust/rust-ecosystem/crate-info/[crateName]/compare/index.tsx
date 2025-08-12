"use client";
import React, { useEffect, useState } from 'react';
import Head from 'next/head';
import Image from 'next/image';
import { useParams } from 'next/navigation';
import { useRouter } from 'next/router';
import { AppLayout } from '@/components/Layout/AppLayout';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
// import { ArrowsRightLeftIcon, ChevronDownIcon } from '@heroicons/react/24/outline';
import CrateInfoLayout from '../layout';

interface VersionData {
    version: string;
    published: string;
    description: string;
    licenses: {
        primary: string;
        dependencies: string[];
    };
    securityAdvisories: {
        inDependencies: Array<{
            id: string;
            description: string;
            severity?: string;
        }>;
    };
    dependencies: Array<{
        name: string;
        version: string;
        highlighted?: 'added' | 'removed' | 'updated';
    }>;
}

const ComparePage = () => {
    const params = useParams();
    const router = useRouter();
    const [leftVersion, setLeftVersion] = useState<VersionData | null>(null);
    const [rightVersion, setRightVersion] = useState<VersionData | null>(null);
    const selectedLeftVersion = '1.2.01';
    const selectedRightVersion = '1.2.01';
    const [isSwapped, setIsSwapped] = useState(false);

    // 从查询参数或URL参数中获取crate信息
    const crateName = (router.query.crateName as string) || params?.crateName as string || "tokio";

    useEffect(() => {
        // 模拟版本数据
        const getMockVersionData = (version: string, isLeftCard: boolean = false): VersionData => ({
            version,
            published: version === '1.2.01' ? 'March7, 2025' : 'March7, 2025',
            description: 'An event-driven,non-blocking I/O platform for writing asynchronous I/O backed applications.',
            licenses: {
                primary: 'MIT',
                dependencies: [
                    '0BSD OR Apache-2.0 OR MIT',
                    ...(version === '1.2.01' ? ['Apache-2.0', 'Apache-2.0 OR BSD-2-Clause OR MIT', 'Apache-2.0 OR BSL-1.0'] : []),
                    'Apache-2.0 OR LGPL-2.1-or-later OR MIT',
                    'Unicode-3.0 AND (Apache-2.0 OR MIT)'
                ]
            },
            securityAdvisories: {
                inDependencies: [
                    {
                        id: 'request',
                        description: 'Server-Side Request Forgery In Request',
                        severity: '2.88.2'
                    },
                    {
                        id: 'GHSA-p8p7-x288-28g6',
                        description: 'Server-Side Request Forgery In Request'
                    },
                    {
                        id: 'tough-cookie',
                        description: 'Server-Side Request Forgery In Request',
                        severity: '2.5.1'
                    },
                    {
                        id: 'GHSA-p8p7-x288-28g6',
                        description: 'Server-Side Request Forgery In Request'
                    }
                ]
            },
            dependencies: [
                { name: 'abab', version: '2.0.6' },
                { name: 'acorn', version: '5.7.4', highlighted: (version === '1.2.01' && isLeftCard) ? 'added' : undefined },
                { name: 'acorn-globals', version: '6.4.2' },
                { name: 'ajv', version: '6.12.6' },
                { name: 'acorn', version: '5.7.4' },
                { name: 'abab', version: '2.0.6' },
                { name: 'browser-process-hrtime-addsvj', version: '6.4.2' }
            ]
        });

        setLeftVersion(getMockVersionData(selectedLeftVersion, true));
        setRightVersion(getMockVersionData(selectedRightVersion, false));
    }, [selectedLeftVersion, selectedRightVersion]);

    const handleSwapVersions = () => {
        setIsSwapped(!isSwapped);
    };

    const VersionSelector = ({ 
        version 
    }: { 
        version: string; 
    }) => (
        <div 
            className="flex items-center space-x-2"
            style={{
                display: 'flex',
                width: '140px',
                height: '40px',
                padding: '0 12px',
                alignItems: 'center',
                gap: '10px',
                borderRadius: '6px',
                border: '1px solid #00062e33',
                background: '#ffffffe6'
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
            case 'added': return 'rgb(234,141,143)'; // 粉色（红色），acorn使用
            case 'removed': return 'rgb(234,141,143)'; // 红色  
            case 'updated': return 'rgb(232,192,97)'; // 橙色
            default: return 'transparent';
        }
    };

    if (!leftVersion || !rightVersion) {
        return <div>Loading...</div>;
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
                                                    color: '#00071b80',
                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                    fontSize: '16px',
                                                    fontStyle: 'normal',
                                                    fontWeight: 400,
                                                    lineHeight: '20px'
                                                }}
                                            >
                                                {leftVersion.licenses.primary}
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
                                                 {leftVersion.licenses.dependencies.map((license) => (
                                                     <div 
                                                         key={`left-license-${license}`}
                                                        className="p-1 text-sm"
                                                                                                                                                                         style={{ 
                                                            backgroundColor: license === 'Apache-2.0' && selectedLeftVersion === '1.2.01' ? 'rgb(234,141,143)' : 'transparent',
                                                            color: license === 'Apache-2.0' && selectedLeftVersion === '1.2.01' ? 'white' : '#00071b80',
                                                            fontFamily: '"HarmonyOS Sans SC"',
                                                            fontSize: '16px',
                                                            fontStyle: 'normal',
                                                            fontWeight: 400,
                                                            lineHeight: '20px',
                                                            marginLeft: license === 'Apache-2.0' && selectedLeftVersion === '1.2.01' ? '-20px' : '0',
                                                            marginRight: license === 'Apache-2.0' && selectedLeftVersion === '1.2.01' ? '-20px' : '0',
                                                            paddingLeft: license === 'Apache-2.0' && selectedLeftVersion === '1.2.01' ? '20px' : '0',
                                                            paddingRight: license === 'Apache-2.0' && selectedLeftVersion === '1.2.01' ? '20px' : '0',
                                                            borderRadius: license === 'Apache-2.0' && selectedLeftVersion === '1.2.01' ? '0' : '4px',
                                                            display: 'flex',
                                                            alignItems: 'center',
                                                            justifyContent: 'space-between'
                                                        }}
                                                    >
                                                        <span>{license}</span>
                                                        {license === 'Apache-2.0' && selectedLeftVersion === '1.2.01' && (
                                                            <svg 
                                                                fill="currentColor" 
                                                                viewBox="0 0 20 20"
                                                                style={{
                                                                    width: '16px',
                                                                    height: '16px',
                                                                    color: 'white'
                                                                }}
                                                            >
                                                                <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                                                            </svg>
                                                        )}
                                                    </div>
                                                ))}
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
                                             {leftVersion.securityAdvisories.inDependencies.map((advisory, index) => (
                                                 <div 
                                                     key={`left-advisory-${advisory.id}-${advisory.description}`}
                                                    className="p-2 rounded"
                                                                                                         style={{ 
                                                         backgroundColor: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? 'rgb(234,141,143)' : 'transparent',
                                                         color: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? 'white' : '#333333',
                                                         display: 'flex',
                                                         alignItems: 'center',
                                                         justifyContent: 'space-between',
                                                         marginLeft: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? '-20px' : '0',
                                                         marginRight: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? '-20px' : '0',
                                                         paddingLeft: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? '20px' : '8px',
                                                         paddingRight: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? '20px' : '8px',
                                                         borderRadius: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? '0' : '4px',
                                                         lineHeight: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? '20px' : 'normal',
                                                         height: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? '30px' : 'auto'
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
                                                                color: advisory.id.startsWith('GHSA-') ? '#002bb7c4' : '#00071b80',
                                                                fontFamily: '"HarmonyOS Sans SC"',
                                                                fontSize: advisory.id.startsWith('GHSA-') ? '14px' : '16px',
                                                                fontStyle: 'normal',
                                                                fontWeight: 400,
                                                                lineHeight: '20px',
                                                                ...(advisory.id.startsWith('GHSA-') && {
                                                                    display: '-webkit-box',
                                                                    WebkitBoxOrient: 'vertical',
                                                                    WebkitLineClamp: 1,
                                                                    overflow: 'hidden',
                                                                    textOverflow: 'ellipsis',
                                                                    letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                                })
                                                            }}
                                                        >
                                                            {advisory.id}
                                                        </div>
                                                        {advisory.severity && (
                                                            <div 
                                                                className="text-sm"
                                                                style={{ 
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    color: '#80838d',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px'
                                                                }}
                                                            >
                                                                {advisory.severity}
                                                            </div>
                                                        )}
                                                        {advisory.id !== 'request' && advisory.id !== 'tough-cookie' && (
                                                            <div 
                                                                className="text-sm text-blue-600 hover:underline cursor-pointer"
                                                                style={{ 
                                                                    color: advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 ? '#fcfcfd' : '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '16px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px',
                                                                    textTransform: 'capitalize',
                                                                    marginLeft: '16px'
                                                                }}
                                                            >
                                                                {advisory.description}
                                                            </div>
                                                        )}
                                                    </div>
                                                    {advisory.id === 'GHSA-p8p7-x288-28g6' && index === 1 && (
                                                        <svg 
                                                            fill="currentColor" 
                                                            viewBox="0 0 20 20"
                                                            style={{
                                                                width: '16px',
                                                                height: '16px',
                                                                color: 'white'
                                                            }}
                                                        >
                                                            <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                                                        </svg>
                                                    )}
                                                </div>
                                            ))}
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
                                                                         <div className="space-y-1">
                                         {leftVersion.dependencies.map((dep) => (
                                                                                          <div 
                                                  key={`left-dep-${dep.name}-${dep.version}`}
                                                 className="flex justify-between items-center p-2 rounded"
                                                 style={{ 
                                                     backgroundColor: getHighlightColor(dep.highlighted),
                                                     color: dep.highlighted ? 'white' : '#333333',
                                                     display: 'flex',
                                                     alignItems: 'center',
                                                     justifyContent: 'space-between',
                                                     marginLeft: dep.highlighted ? '-20px' : '0',
                                                     marginRight: dep.highlighted ? '-20px' : '0',
                                                     paddingLeft: dep.highlighted ? '20px' : '8px',
                                                     paddingRight: dep.highlighted ? '20px' : '8px',
                                                     borderRadius: dep.highlighted ? '0' : '4px',
                                                     lineHeight: dep.highlighted ? '20px' : 'normal',
                                                     height: dep.highlighted ? '30px' : 'auto'
                                                 }}
                                             >
                                                <div className="flex justify-between items-center flex-1">
                                                    <span 
                                                        className="text-sm cursor-pointer hover:underline"
                                                        style={{ 
                                                            color: dep.highlighted ? 'white' : '#00071b80',
                                                            fontFamily: '"HarmonyOS Sans SC"',
                                                            fontSize: '16px',
                                                            fontStyle: 'normal',
                                                            fontWeight: 400,
                                                            lineHeight: '20px'
                                                        }}
                                                    >
                                                        {dep.name}
                                                    </span>
                                                    <span 
                                                        className="text-sm"
                                                        style={{ 
                                                            color: dep.highlighted ? '#ffffff' : '#00071b80',
                                                            fontFamily: '"HarmonyOS Sans SC"',
                                                            fontSize: '16px',
                                                            fontStyle: 'normal',
                                                            fontWeight: 400,
                                                            lineHeight: '20px'
                                                        }}
                                                    >
                                                        {dep.version}
                                                    </span>
                                                </div>
                                                {dep.highlighted && (
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
                                        ))}
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
                                                    color: '#00071b80',
                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                    fontSize: '16px',
                                                    fontStyle: 'normal',
                                                    fontWeight: 400,
                                                    lineHeight: '20px'
                                                }}
                                            >
                                                {rightVersion.licenses.primary}
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
                                                 {rightVersion.licenses.dependencies.map((license) => (
                                                     <div 
                                                         key={`right-license-${license}`}
                                                        className="p-1 text-sm"
                                                        style={{ 
                                                            backgroundColor: license === 'Apache-2.0 OR BSL-1.0' && selectedRightVersion === '1.2.01' ? 'rgb(232,192,97)' : 'transparent',
                                                            color: license === 'Apache-2.0 OR BSL-1.0' && selectedRightVersion === '1.2.01' ? '#ffffff' : '#00071b80',
                                                            fontFamily: '"HarmonyOS Sans SC"',
                                                            fontSize: '16px',
                                                            fontStyle: 'normal',
                                                            fontWeight: 400,
                                                            lineHeight: '20px',
                                                            marginLeft: '-20px',
                                                            marginRight: '-20px',
                                                            paddingLeft: '20px',
                                                            paddingRight: '20px',
                                                            borderRadius: '0',
                                                            display: 'flex',
                                                            alignItems: 'center',
                                                            justifyContent: 'space-between'
                                                        }}
                                                    >
                                                        <span>{license}</span>
                                                                                                                 {license === 'Apache-2.0 OR BSL-1.0' && selectedRightVersion === '1.2.01' && (
                                                             <svg 
                                                                 fill="currentColor" 
                                                                 viewBox="0 0 20 20"
                                                                 style={{
                                                                     width: '16px',
                                                                     height: '16px',
                                                                     color: 'white'
                                                                 }}
                                                             >
                                                                 <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                                                             </svg>
                                                         )}
                                                    </div>
                                                ))}
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
                                             {rightVersion.securityAdvisories.inDependencies.map((advisory) => (
                                                 <div 
                                                     key={`right-advisory-${advisory.id}-${advisory.description}`}
                                                    className="p-2 rounded"
                                                    style={{ 
                                                        backgroundColor: 'transparent',
                                                        color: '#333333'
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
                                                                color: advisory.id.startsWith('GHSA-') ? '#002bb7c4' : '#00071b80',
                                                                fontFamily: '"HarmonyOS Sans SC"',
                                                                fontSize: advisory.id.startsWith('GHSA-') ? '14px' : '16px',
                                                                fontStyle: 'normal',
                                                                fontWeight: 400,
                                                                lineHeight: '20px',
                                                                ...(advisory.id.startsWith('GHSA-') && {
                                                                    display: '-webkit-box',
                                                                    WebkitBoxOrient: 'vertical',
                                                                    WebkitLineClamp: 1,
                                                                    overflow: 'hidden',
                                                                    textOverflow: 'ellipsis',
                                                                    letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
                                                                })
                                                            }}
                                                        >
                                                            {advisory.id}
                                                        </div>
                                                        {advisory.severity && (
                                                            <div 
                                                                className="text-sm"
                                                                style={{ 
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    color: '#80838d',
                                                                    fontSize: '14px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px'
                                                                }}
                                                            >
                                                                {advisory.severity}
                                                            </div>
                                                        )}
                                                        {advisory.id !== 'request' && advisory.id !== 'tough-cookie' && (
                                                            <div 
                                                                className="text-sm text-blue-600 hover:underline cursor-pointer"
                                                                style={{ 
                                                                    color: '#00071b80',
                                                                    fontFamily: '"HarmonyOS Sans SC"',
                                                                    fontSize: '16px',
                                                                    fontStyle: 'normal',
                                                                    fontWeight: 400,
                                                                    lineHeight: '20px',
                                                                    textTransform: 'capitalize',
                                                                    marginLeft: '16px'
                                                                }}
                                                            >
                                                                {advisory.description}
                                                            </div>
                                                        )}
                                                    </div>
                                                </div>
                                            ))}
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
                                                                         <div className="space-y-1">
                                         {rightVersion.dependencies.map((dep) => (
                                             <div 
                                                 key={`right-dep-${dep.name}-${dep.version}`}
                                                className="flex justify-between items-center p-2 rounded"
                                                style={{ 
                                                    backgroundColor: 'transparent',
                                                    color: '#333333',
                                                    display: 'flex',
                                                    alignItems: 'center',
                                                    justifyContent: 'space-between',
                                                    padding: '8px'
                                                }}
                                            >
                                                <div className="flex justify-between items-center flex-1">
                                                    <span 
                                                        className="text-sm cursor-pointer hover:underline"
                                                        style={{ 
                                                            color: dep.highlighted ? 'white' : '#00071b80',
                                                            fontFamily: '"HarmonyOS Sans SC"',
                                                            fontSize: '16px',
                                                            fontStyle: 'normal',
                                                            fontWeight: 400,
                                                            lineHeight: '20px'
                                                        }}
                                                    >
                                                        {dep.name}
                                                    </span>
                                                    <span 
                                                        className="text-sm"
                                                        style={{ 
                                                            color: dep.highlighted ? '#ffffff' : '#00071b80',
                                                            fontFamily: '"HarmonyOS Sans SC"',
                                                            fontSize: '16px',
                                                            fontStyle: 'normal',
                                                            fontWeight: 400,
                                                            lineHeight: '20px'
                                                        }}
                                                    >
                                                        {dep.version}
                                                    </span>
                                                </div>
                                            </div>
                                        ))}
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
