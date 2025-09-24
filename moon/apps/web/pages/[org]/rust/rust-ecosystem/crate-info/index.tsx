// crate-info页面
"use client";
import React, { useEffect, useState } from 'react';
import { Link } from '@gitmono/ui/Link';
import Head from 'next/head';
import { useParams } from 'next/navigation';
import { useRouter } from 'next/router';
// import Image from 'next/image';
import { AppLayout } from '@/components/Layout/AppLayout';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
// import { MagnifyingGlassIcon } from '@heroicons/react/24/outline';
import CrateInfoLayout from './layout';

// interface CVE {
//     subtitle?: string;
//     id?: string;
//     reported?: string;
//     issued?: string;
//     package?: string;
//     ttype?: string;
//     aliases?: string | string[];
//     keywords?: string;
//     patched?: string;
//     unaffected?: string;
//     url?: string;
//     reference?: string;
//     description?: string;
// }

export interface cratesInfo {
    "crate_name": string,
    "description": string,
    "dependencies": {
        "direct": number,
        "indirect": number
    },
    "dependents": {
        "direct": number,
        "indirect": number
    },
    "cves": Array<{
        "id": string,
        "subtitle": string,
        "reported": string,
        "issued": string,
        "package": string,
        "ttype": string,
        "keywords": string,
        "aliases": string,
        "reference": string,
        "patched": string,
        "unaffected": string,
        "description": string,
        "url": string,
    }>,
    "dep_cves": Array<{
        "id": string,
        "subtitle": string,
        "reported": string,
        "issued": string,
        "package": string,
        "ttype": string,
        "keywords": string,
        "aliases": string,
        "reference": string,
        "patched": string,
        "unaffected": string,
        "description": string,
        "url": string,
    }>,
    "license": string,
    "github_url": string,
    "doc_url": string,
    "versions": string[],
}


const CratePage = () => {
    const params = useParams();
    const router = useRouter();
    const [results, setResults] = useState<cratesInfo | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [_packageCurrentPage, _setPackageCurrentPage] = useState(1);
    const [_depCurrentPage, _setDepCurrentPage] = useState(1);
    const [versions, setVersions] = useState<string[]>([]);
    const [senseleakData, setSenseleakData] = useState<any>(null);
    const [senseleakLoading, setSenseleakLoading] = useState(false);
    const [senseleakError, setSenseleakError] = useState<string | null>(null);
    const [unsafecheckerData, setUnsafecheckerData] = useState<any>(null);
    const [unsafecheckerLoading, setUnsafecheckerLoading] = useState(false);
    const [unsafecheckerError, setUnsafecheckerError] = useState<string | null>(null);
    // const itemsPerPage = 1;

    // 从查询参数或URL参数中获取crate信息
    const crateName = (router.query.crateName as string) || params?.crateName as string || "example-crate";
    const version = (router.query.version as string) || params?.version as string || "1.0.0";
    const nsfront = (router.query.nsfront as string) || params?.nsfront as string || router.query.org as string;
    const nsbehind = (router.query.nsbehind as string) || params?.nsbehind as string || "rust/rust-ecosystem/crate-info";
    const name = crateName;
    

    
    // const basePath = `/${nsfront}/${nsbehind}/${name}/${version}`;

    // 获取 Senseleak 数据
    useEffect(() => {
        const fetchSenseleakData = async () => {
            try {
                setSenseleakLoading(true);
                setSenseleakError(null);
                const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL

                const response = await fetch(`${apiBaseUrl}/api/crates/${nsfront}/${nsbehind}/${crateName}/${version}/senseleak`);
                
                if (!response.ok) {
                    throw new Error('Failed to fetch senseleak data');
                }
                
                const data = await response.json();

                setSenseleakData(data);
            } catch (err) {
                setSenseleakError('Failed to load senseleak data');
            } finally {
                setSenseleakLoading(false);
            }
        };

        if (crateName && version && nsfront && nsbehind) {
            fetchSenseleakData();
        }
    }, [crateName, version, nsfront, nsbehind]);

    // 获取 Unsafechecker 数据
    useEffect(() => {
        const fetchUnsafecheckerData = async () => {
            try {
                setUnsafecheckerLoading(true);
                setUnsafecheckerError(null);
                
                const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL
                const response = await fetch(`${apiBaseUrl}/api/crates/${nsfront}/${nsbehind}/${crateName}/${version}/unsafechecker`);
                
                if (!response.ok) {
                    throw new Error('Failed to fetch unsafechecker data');
                }
                
                const data = await response.json();

                setUnsafecheckerData(data);
            } catch (err) {
                setUnsafecheckerError('Failed to load unsafechecker data');
            } finally {
                setUnsafecheckerLoading(false);
            }
        };

        if (crateName && version && nsfront && nsbehind) {
            fetchUnsafecheckerData();
        }
    }, [crateName, version, nsfront, nsbehind]);

    // 获取crate信息
    useEffect(() => {
        const fetchCrateInfo = async () => {
            try {
                setLoading(true);
                setError(null);
                const apiBaseUrl = process.env.NEXT_PUBLIC_CRATES_PRO_URL;
                
                const response = await fetch(`${apiBaseUrl}/api/crates/${nsfront}/${nsbehind}/${crateName}/${version}`);
                
                if (!response.ok) {
                    throw new Error('Failed to fetch crate information');
                }
                
                const data: cratesInfo = await response.json();

                setResults(data);
                setVersions(data.versions);
            } catch (err) {
                setError('Failed to load crate information');
            } finally {
                setLoading(false);
            }
        };

        if (crateName && version && nsfront && nsbehind) {
            fetchCrateInfo();
        }
    }, [crateName, version, nsfront, nsbehind]);

    if (loading) {
        return (
            <div className="flex justify-center items-center min-h-screen">
                <div className="text-gray-500">加载中...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="flex justify-center items-center min-h-screen">
                <div className="text-red-500">错误: {error}</div>
            </div>
        );
    }

    if (!results) {
        return (
            <div className="flex justify-center items-center min-h-screen">
                <div className="text-gray-500">未找到crate信息</div>
            </div>
        );
    }

    // const _getCurrentPageItems = (items: CVE[], currentPage: number) => {

    //     const start = (currentPage - 1) * itemsPerPage;

    //     const end = start + itemsPerPage;

    //     return items.slice(start, end);
    // };

    return (
        <>
            <Head>
                <title>Crate Info - {crateName || 'Crate'}</title>
            </Head>
            <CrateInfoLayout versions={versions}>
                <div className="flex justify-center pb-8">
                    <div className="w-[1370px] px-8 py-4">
                        <div className="grid grid-cols-1 lg:grid-cols-3 gap-12">
                            {/* 左侧内容区域 - 占据2列 */}
                            <div className="lg:col-span-2 space-y-6" style={{ width: '800px' }}>
                                                                            {/* Security Advisories 内容 */}
                                 <div className="bg-white rounded-2xl p-6 shadow-[0_0_12px_0_rgba(43,88,221,0.09)]">
                                     {/* 卡片头部 */}
                                     <div className="flex justify-between items-center mb-6">
                                         <div>
                                             <h3 className="text-[24px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-medium tracking-[0.96px]">Security Advisories</h3>
                                                <p 
                                                  className="mt-3"
                                                  style={{
                                                      alignSelf: 'stretch',
                                                      color: '#1c2024',
                                                      fontFamily: '"HarmonyOS Sans SC"',
                                                      fontSize: '20px',
                                                      fontStyle: 'normal',
                                                      fontWeight: 400,
                                                      lineHeight: '16px',
                                                      letterSpacing: '0.04px'
                                                  }}
                                              >
                                                  In the dependencies
                                              </p>
                                         </div>
                                                                                   <span 
                                              className="flex-shrink-0 text-sm text-white"
                                              style={{
                                                  display: 'flex',
                                                  width: '33px',
                                                  height: '33px',
                                                  flexDirection: 'column',
                                                  justifyContent: 'center',
                                                  alignItems: 'center',
                                                  aspectRatio: '1/1',
                                                  borderRadius: '6px',
                                                  background: '#E5484D'
                                              }}
                                          >
                                              {(results?.cves?.length || 0) + (results?.dep_cves?.length || 0)}
                                          </span>
                                     </div>

                                     {/* 安全公告列表 */}
                                     <div className="space-y-4">
                                         {/* CVE列表 */}
                                         {results.cves.slice(0, 3).map((cve) => (
                                             <div key={cve.id} className="flex justify-between items-start py-3 border-b border-gray-100">
                                                 <div className="flex-1">
                                                     <p className="text-[#FD5656] font-['HarmonyOS_Sans_SC'] text-[16px] font-normal leading-[18px] mb-1">
                                                         {cve.subtitle || cve.description}
                                                     </p>
                                                     <p className="text-[#666666] font-['HarmonyOS_Sans_SC'] text-[14px] font-normal">
                                                         {cve.id}
                                                     </p>
                                                 </div>
                                                 <button className="ml-4 px-4 py-2 border border-[#4B68FF] text-[#4B68FF] text-[14px] font-['HarmonyOS_Sans_SC'] font-normal rounded hover:bg-[#4B68FF] hover:text-white transition-colors">
                                                     MORE DETAILS
                                                 </button>
                                             </div>
                                         ))}

                                         {/* SIMILAR ADVISORIES 标题 */}
                                         <div className="py-1 pl-6">
                                             <p className="text-[#666666] font-['HarmonyOS_Sans_SC'] text-[12px] font-normal uppercase tracking-wide">
                                                 SIMILAR ADVISORIES
                                             </p>
                                         </div>

                                         {/* 依赖CVE列表 */}
                                         {results.dep_cves.slice(0, 2).map((cve) => (
                                             <div key={cve.id} className="flex justify-between items-start py-3 border-b border-gray-100 pl-6">
                                                 <div className="flex-1">
                                                     <p className="text-[#FD5656] font-['HarmonyOS_Sans_SC'] text-[16px] font-normal leading-[18px] mb-1">
                                                         {cve.subtitle || cve.description}
                                                     </p>
                                                     <p className="text-[#666666] font-['HarmonyOS_Sans_SC'] text-[14px] font-normal">
                                                         {cve.id}
                                                     </p>
                                                 </div>
                                                 <button className="ml-4 px-4 py-2 border border-[#4B68FF] text-[#4B68FF] text-[14px] font-['HarmonyOS_Sans_SC'] font-normal rounded hover:bg-[#4B68FF] hover:text-white transition-colors">
                                                     MORE DETAILS
                                                 </button>
                                             </div>
                                         ))}
                                     </div>
                                 </div>

                                {/* Licenses */}
                                <div className="space-y-6">
                                    {/* Licenses 标题 */}
                                        {/* <div className="flex justify-between items-center">
                                            <div className="flex items-center gap-3">
                                                <div className="w-[4px] h-[24px] flex-shrink-0 rounded-[2px] bg-[#4B68FF]"></div>
                                                <h2 className="text-[24px] font-bold text-[#333333] tracking-[0.96px] font-['HarmonyOS_Sans_SC']">
                                                    Licenses
                                                </h2>
                                            </div>
                                        </div> */}
                                    {/* Licenses 内容 */}
                                    <div className="bg-white rounded-2xl p-6 shadow-[0_0_12px_0_rgba(43,88,221,0.09)]">
                                        {/* 卡片头部 */}
                                        <div className="flex justify-between items-center mb-6">
                                                                                                                                      <div>
                                                 <h3 className="text-[24px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-medium tracking-[0.96px]">Licenses</h3>
                                                 <p 
                                                   className="mt-3"
                                                   style={{
                                                       alignSelf: 'stretch',
                                                       color: '#1c2024',
                                                       fontFamily: '"HarmonyOS Sans SC"',
                                                       fontSize: '20px',
                                                       fontStyle: 'normal',
                                                       fontWeight: 400,
                                                       lineHeight: '16px',
                                                       letterSpacing: '0.04px'
                                                   }}
                                                 >
                                                     In the dependencies
                                                 </p>
                                                 <span className="text-[#4B68FF] text-[14px] font-['HarmonyOS_Sans_SC'] font-normal hover:underline cursor-pointer mt-2 block">
                                                     Learn more about license information.
                                                 </span>
                                             </div>
                                            <span 
                                                className="flex-shrink-0 text-sm text-white"
                                                style={{
                                                    display: 'flex',
                                                    width: '33px',
                                                    height: '33px',
                                                    flexDirection: 'column',
                                                    justifyContent: 'center',
                                                    alignItems: 'center',
                                                    aspectRatio: '1/1',
                                                    borderRadius: '6px',
                                                    background: '#4B68FF'
                                                }}
                                            >
                                                3
                                            </span>
                                        </div>

                                        {/* 主许可证部分 */}
                                        <div className="mb-6">
                                            <p className="text-[#666666] font-['HarmonyOS_Sans_SC'] text-[12px] font-normal uppercase tracking-wide mb-2">
                                                LICENSES
                                            </p>
                                            <div className="text-[36px] font-['HarmonyOS_Sans_SC'] font-bold text-[#333333]">
                                                {results.license || 'Unknown'}
                                            </div>
                                        </div>

                                                                                 {/* 依赖许可证部分 */}
                                         <div>
                                             <p className="text-[#666666] font-['HarmonyOS_Sans_SC'] text-[12px] font-normal uppercase tracking-wide mb-4">
                                                 DEPENDENCY LICENSES
                                             </p>
                                             <div className="space-y-4">
                                                                                                   {/* MIT */}
                                                  <div className="grid grid-cols-[80px_48px_1fr] gap-3 items-center">
                                                      <div 
                                                          className="capitalize"
                                                          style={{
                                                              color: '#002bb7c4',
                                                              fontFamily: '"HarmonyOS Sans SC"',
                                                              fontSize: '14px',
                                                              fontStyle: 'normal',
                                                              fontWeight: 400,
                                                              lineHeight: 'normal',
                                                              letterSpacing: 0
                                                          }}
                                                      >
                                                          MIT
                                                      </div>
                                                                                                             <div className="text-right text-[#4B68FF] text-[18px] font-['HarmonyOS_Sans_SC'] font-normal capitalize">77</div>
                                                       <div className="h-2 rounded-lg overflow-hidden bg-[#F5F7FF]" style={{ width: '482px' }}>
                                                           <div
                                                               className="h-full bg-[#4B68FF] rounded-lg"
                                                               style={{ width: '85%' }}
                                                           />
                                                       </div>
                                                  </div>
                                                  
                                                  {/* BSD-2-Clause */}
                                                  <div className="grid grid-cols-[80px_48px_1fr] gap-3 items-center">
                                                      <div 
                                                          className="capitalize"
                                                          style={{
                                                              color: '#002bb7c4',
                                                              fontFamily: '"HarmonyOS Sans SC"',
                                                              fontSize: '14px',
                                                              fontStyle: 'normal',
                                                              fontWeight: 400,
                                                              lineHeight: 'normal',
                                                              letterSpacing: 0
                                                          }}
                                                      >
                                                          BSD-2-Clause
                                                      </div>
                                                  <div className="text-right text-[#4B68FF] text-[18px] font-['HarmonyOS_Sans_SC'] font-normal capitalize">55</div>
                                                       <div className="h-2 rounded-lg overflow-hidden bg-[#F5F7FF]" style={{ width: '482px' }}>
                                                           <div
                                                               className="h-full bg-[#4B68FF] rounded-lg"
                                                               style={{ width: '60%' }}
                                                           />
                                                       </div>
                                                  </div>
                                                  
                                                  {/* ISC */}
                                                  <div className="grid grid-cols-[80px_48px_1fr] gap-3 items-center">
                                                      <div 
                                                          className="capitalize"
                                                          style={{
                                                              color: '#002bb7c4',
                                                              fontFamily: '"HarmonyOS Sans SC"',
                                                              fontSize: '14px',
                                                              fontStyle: 'normal',
                                                              fontWeight: 400,
                                                              lineHeight: 'normal',
                                                              letterSpacing: 0
                                                          }}
                                                      >
                                                          ISC
                                                      </div>
                                                   <div className="text-right text-[#4B68FF] text-[18px] font-['HarmonyOS_Sans_SC'] font-normal capitalize">22</div>
                                                       <div className="h-2 rounded-lg overflow-hidden bg-[#F5F7FF]" style={{ width: '482px' }}>
                                                           <div
                                                               className="h-full bg-[#4B68FF] rounded-lg"
                                                               style={{ width: '25%' }}
                                                           />
                                                       </div>
                                                  </div>
                                             </div>
                                         </div>
                                    </div>
                                </div>

                            {/* Dependencies */}
                                 <div className="space-y-6">
                                     {/* Dependencies 内容 */}
                                     {results && results.dependencies && (results.dependencies.direct + results.dependencies.indirect) > 0 ? (
                                         <div className="bg-white rounded-2xl p-6 shadow-[0_0_12px_0_rgba(43,88,221,0.09)]">
                                             {/* 卡片头部 */}
                                             <div className="flex justify-between items-center mb-6">
                                                 <div>
                                                     <h3 className="text-[24px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-medium tracking-[0.96px]">Dependencies</h3>
                                                 </div>
                                                 <span 
                                                     className="flex-shrink-0 text-sm text-white"
                                                     style={{
                                                         display: 'flex',
                                                         width: '33px',
                                                         height: '33px',
                                                         flexDirection: 'column',
                                                         justifyContent: 'center',
                                                         alignItems: 'center',
                                                         aspectRatio: '1/1',
                                                         borderRadius: '6px',
                                                         background: '#4B68FF'
                                                     }}
                                                 >
                                                     {results.dependencies.direct + results.dependencies.indirect}
                                                 </span>
                                             </div>
                                             <div className="space-y-4">
                                                                                                 {/* Direct */}
                                                 <div className="grid grid-cols-[80px_48px_1fr] gap-3 items-center">
                                                     <div 
                                                         className="capitalize"
                                                         style={{
                                                             color: '#002bb7c4',
                                                             fontFamily: '"HarmonyOS Sans SC"',
                                                             fontSize: '14px',
                                                             fontStyle: 'normal',
                                                             fontWeight: 400,
                                                             lineHeight: 'normal',
                                                             letterSpacing: 0
                                                         }}
                                                     >
                                                         Direct
                                                     </div>
                                                                                                           <div className="text-right text-[#4B68FF] text-[18px] font-['HarmonyOS_Sans_SC'] font-normal capitalize">{results.dependencies.direct}</div>
                                                      <div className="h-2 rounded-lg overflow-hidden bg-[#F5F7FF]" style={{ width: '482px' }}>
                                                          <div
                                                              className="h-full bg-[#4B68FF] rounded-lg"
                                                              style={{
                                                                  width: `${(results.dependencies.direct / (results.dependencies.direct + results.dependencies.indirect)) * 100}%`
                                                              }}
                                                          />
                                                      </div>
                                                 </div>

                                                 {/* Indirect */}
                                                 <div className="grid grid-cols-[80px_48px_1fr] gap-3 items-center">
                                                     <div 
                                                         className="capitalize"
                                                         style={{
                                                             color: '#002bb7c4',
                                                             fontFamily: '"HarmonyOS Sans SC"',
                                                             fontSize: '14px',
                                                             fontStyle: 'normal',
                                                             fontWeight: 400,
                                                             lineHeight: 'normal',
                                                             letterSpacing: 0
                                                         }}
                                                     >
                                                         Indirect
                                                     </div>
                                                                                                           <div className="text-right text-[#4B68FF] text-[18px] font-['HarmonyOS_Sans_SC'] font-normal capitalize">{results.dependencies.indirect}</div>
                                                      <div className="h-2 rounded-lg overflow-hidden bg-[#F5F7FF]" style={{ width: '482px' }}>
                                                          <div
                                                              className="h-full bg-[#4B68FF] rounded-lg"
                                                              style={{
                                                                  width: `${(results.dependencies.indirect / (results.dependencies.direct + results.dependencies.indirect)) * 100}%`
                                                              }}
                                                          />
                                                      </div>
                                                 </div>
                                            </div>

                                            <div className="mt-6 text-center">
                                                <Link href={`/${nsfront}/rust/rust-ecosystem/crate-info/${crateName}/dependencies?crateName=${crateName}&version=${version}`}>
                                                    <span className="text-[#4B68FF] text-[18px] font-['HarmonyOS_Sans_SC'] font-normal hover:underline">
                                                        View all dependencies ({results.dependencies.direct + results.dependencies.indirect})
                                                    </span>
                                                </Link>
                                            </div>
                                        </div>
                                    ) : (
                                        <div className="text-[#333333] font-['HarmonyOS_Sans_SC'] text-[18px] font-normal leading-normal capitalize">
                                            This Package Has No Known Dependencies.
                                        </div>
                                    )}
                                </div>

                                                                 {/* Dependents */}
                                 <div className="space-y-6">
                                     {/* Dependents 内容 */}
                                     {results && results.dependents && (results.dependents.direct + results.dependents.indirect) > 0 ? (
                                         <div className="bg-white rounded-2xl p-6 shadow-[0_0_12px_0_rgba(43,88,221,0.09)]">
                                             {/* 卡片头部 */}
                                             <div className="flex justify-between items-center mb-6">
                                                 <div>
                                                     <h3 className="text-[24px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-medium tracking-[0.96px]">Dependents</h3>
                                                 </div>
                                                 <span 
                                                     className="flex-shrink-0 text-sm text-white"
                                                     style={{
                                                         display: 'flex',
                                                         width: '33px',
                                                         height: '33px',
                                                         flexDirection: 'column',
                                                         justifyContent: 'center',
                                                         alignItems: 'center',
                                                         aspectRatio: '1/1',
                                                         borderRadius: '6px',
                                                         background: '#4B68FF'
                                                     }}
                                                 >
                                                     {results.dependents.direct + results.dependents.indirect}
                                                 </span>
                                             </div>
                                             <div className="space-y-4">
                                                                                                 {/* Direct */}
                                                 <div className="grid grid-cols-[80px_48px_1fr] gap-3 items-center">
                                                     <div 
                                                         className="capitalize"
                                                         style={{
                                                             color: '#002bb7c4',
                                                             fontFamily: '"HarmonyOS Sans SC"',
                                                             fontSize: '14px',
                                                             fontStyle: 'normal',
                                                             fontWeight: 400,
                                                             lineHeight: 'normal',
                                                             letterSpacing: 0
                                                         }}
                                                     >
                                                         Direct
                                                     </div>
                                                                                                           <div className="text-right text-[#4B68FF] text-[18px] font-['HarmonyOS_Sans_SC'] font-normal capitalize">{results.dependents.direct}</div>
                                                      <div className="h-2 rounded-lg overflow-hidden bg-[#F5F7FF]" style={{ width: '482px' }}>
                                                          <div
                                                              className="h-full bg-[#4B68FF] rounded-lg"
                                                              style={{
                                                                  width: `${(results.dependents.direct / (results.dependents.direct + results.dependents.indirect)) * 100}%`
                                                              }}
                                                          />
                                                      </div>
                                                 </div>

                                                 {/* Indirect */}
                                                 <div className="grid grid-cols-[80px_48px_1fr] gap-3 items-center">
                                                     <div 
                                                         className="capitalize"
                                                         style={{
                                                             color: '#002bb7c4',
                                                             fontFamily: '"HarmonyOS Sans SC"',
                                                             fontSize: '14px',
                                                             fontStyle: 'normal',
                                                             fontWeight: 400,
                                                             lineHeight: 'normal',
                                                             letterSpacing: 0
                                                         }}
                                                     >
                                                         Indirect
                                                     </div>
                                                                                                           <div className="text-right text-[#4B68FF] text-[18px] font-['HarmonyOS_Sans_SC'] font-normal capitalize">{results.dependents.indirect}</div>
                                                      <div className="h-2 rounded-lg overflow-hidden bg-[#F5F7FF]" style={{ width: '482px' }}>
                                                          <div
                                                              className="h-full bg-[#4B68FF] rounded-lg"
                                                              style={{
                                                                  width: `${(results.dependents.indirect / (results.dependents.direct + results.dependents.indirect)) * 100}%`
                                                              }}
                                                          />
                                                      </div>
                                                 </div>
                                            </div>

                                            <div className="mt-6 text-center">
                                                <Link href={`/${nsfront}/${nsbehind}/${name}/${version}/dependents`}>
                                                    <span className="text-[#4B68FF] text-[18px] font-['HarmonyOS_Sans_SC'] font-normal hover:underline">
                                                        View all dependents ({results.dependents.direct + results.dependents.indirect})
                                                    </span>
                                                </Link>
                                            </div>
                                        </div>
                                    ) : (
                                        <div className="text-[#333333] font-['HarmonyOS_Sans_SC'] text-[18px] font-normal leading-normal capitalize">
                                            This Package Has No Known Dependents.
                                        </div>
                                    )}
                                </div>
                            </div>

                            {/* 右侧内容区域 - 占据1列 */}
                            <div className="space-y-6">
                                {/* Published */}
                                <div>
                                    <h3 className="text-[18px] font-bold text-[#333333] tracking-[0.72px] font-['HarmonyOS_Sans_SC'] mb-2">
                                        Published
                                    </h3>
                                    <p className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">
                                        May 24, 2025
                                    </p>
                                </div>

                                {/* Description */}
                                <div>
                                    <h3 className="text-[18px] font-bold text-[#333333] tracking-[0.72px] font-['HarmonyOS_Sans_SC'] mb-2">
                                        Description
                                    </h3>
                                    <p className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">
                                        {results.description || 'No description available'}
                                    </p>
                                </div>

                                {/* Senseleak */}
                                <div>
                                    <h3 className="text-[18px] font-bold text-[#333333] tracking-[0.72px] font-['HarmonyOS_Sans_SC'] mb-2">
                                        Senseleak
                                    </h3>
                                    {senseleakLoading ? (
                                        <p className="text-[14px] text-[#666666] font-['HarmonyOS_Sans_SC'] font-normal">
                                            Loading...
                                        </p>
                                    ) : senseleakError ? (
                                        <p className="text-[14px] text-[#FD5656] font-['HarmonyOS_Sans_SC'] font-normal">
                                            {senseleakError}
                                        </p>
                                    ) : senseleakData ? (
                                        <div className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">
                                            <pre className="whitespace-pre-wrap break-words">
                                                {JSON.stringify(senseleakData, null, 2)}
                                            </pre>
                                        </div>
                                    ) : (
                                        <p className="text-[14px] text-[#666666] font-['HarmonyOS_Sans_SC'] font-normal">
                                            No data available
                                        </p>
                                    )}
                                </div>

                                {/* Unsafechecker */}
                                <div>
                                    <h3 className="text-[18px] font-bold text-[#333333] tracking-[0.72px] font-['HarmonyOS_Sans_SC'] mb-2">
                                        Unsafechecker
                                    </h3>
                                    {unsafecheckerLoading ? (
                                        <p className="text-[14px] text-[#666666] font-['HarmonyOS_Sans_SC'] font-normal">
                                            Loading...
                                        </p>
                                    ) : unsafecheckerError ? (
                                        <p className="text-[14px] text-[#FD5656] font-['HarmonyOS_Sans_SC'] font-normal">
                                            {unsafecheckerError}
                                        </p>
                                    ) : unsafecheckerData ? (
                                        <div className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">
                                            <pre className="whitespace-pre-wrap break-words">
                                                {JSON.stringify(unsafecheckerData, null, 2)}
                                            </pre>
                                        </div>
                                    ) : (
                                        <p className="text-[14px] text-[#666666] font-['HarmonyOS_Sans_SC'] font-normal">
                                            No data available
                                        </p>
                                    )}
                                </div>

                                {/* Owners */}
                                <div>
                                    <h3 className="text-[18px] font-bold text-[#333333] tracking-[0.72px] font-['HarmonyOS_Sans_SC'] mb-2">
                                        Owners
                                    </h3>
                                    {results.github_url ? (
                                        <a
                                            href={results.github_url}
                                            className="text-[#4B68FF] text-[14px] font-['HarmonyOS_Sans_SC'] font-normal hover:underline break-all"
                                            target="_blank"
                                            rel="noopener noreferrer"
                                        >
                                            {results.github_url}
                                        </a>
                                    ) : (
                                        <p className="text-[14px] text-[#666666] font-['HarmonyOS_Sans_SC'] font-normal">
                                            No GitHub URL available
                                        </p>
                                    )}
                                </div>

                                {/* Links */}
                                <div>
                                    <h3 className="text-[18px] font-bold text-[#333333] tracking-[0.72px] font-['HarmonyOS_Sans_SC'] mb-2">
                                        Links
                                    </h3>
                                    <div className="space-y-2">
                                        {results.github_url && (
                                            <div>
                                                <span className="text-[14px] text-[#666666] font-['HarmonyOS_Sans_SC'] font-normal">REPO:</span>
                                                <a
                                                    href={results.github_url}
                                                    className="text-[#4B68FF] text-[14px] font-['HarmonyOS_Sans_SC'] font-normal hover:underline block break-all"
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                >
                                                    {results.github_url}
                                                </a>
                                            </div>
                                        )}
                                        {results.doc_url && (
                                            <div>
                                                <span className="text-[14px] text-[#666666] font-['HarmonyOS_Sans_SC'] font-normal">DOCS:</span>
                                                <a
                                                    href={results.doc_url}
                                                    className="text-[#4B68FF] text-[14px] font-['HarmonyOS_Sans_SC'] font-normal hover:underline block break-all"
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                >
                                                    {results.doc_url}
                                                </a>
                                            </div>
                                        )}
                                        {!results.github_url && !results.doc_url && (
                                            <p className="text-[14px] text-[#666666] font-['HarmonyOS_Sans_SC'] font-normal">
                                                No links available
                                            </p>
                                        )}
                                    </div>
                                </div>

                                {/* Projects */}
                                <div>
                                    <h3 className="text-[18px] font-bold text-[#333333] tracking-[0.72px] font-['HarmonyOS_Sans_SC'] mb-2">
                                        Projects
                                    </h3>
                                    <div className="space-y-2">
                                        <p className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">
                                            tokio-rs/tokio
                                        </p>
                                        <p className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">
                                            GitHub
                                        </p>
                                        <p className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">
                                            Web scraping made simple.
                                        </p>
                                        <div className="flex gap-2 mt-3">
                                            <button className="flex items-center gap-1 px-3 py-1 bg-[#E3F2FD] text-[#1976D2] text-[12px] font-['HarmonyOS_Sans_SC'] font-normal rounded">
                                                <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                                                    <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                                                </svg>
                                                3K forks
                                            </button>
                                            <button className="flex items-center gap-1 px-3 py-1 bg-[#E3F2FD] text-[#1976D2] text-[12px] font-['HarmonyOS_Sans_SC'] font-normal rounded">
                                                <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                                                    <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
                                                </svg>
                                                29K stars
                                            </button>
                                        </div>
                                    </div>
                                </div>

                                {/* OpenSSF Information */}
                                <div>
                                    <p className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal mb-4">
                                        The Open Source Security Foundation is a cross-industry collaboration to improve the security of
                                        open source software (OSS). The <strong>Scorecard</strong> provides security health metrics for open source projects.
                                    </p>
                                    <a href="#" className="text-[#4B68FF] text-[14px] font-['HarmonyOS_Sans_SC'] font-normal hover:underline block mb-4">
                                        View information about checks and how to fix failures.
                                    </a>
                                </div>

                                {/* SCORE */}
                                <div>
                                    <h3 className="text-[18px] font-bold text-[#333333] tracking-[0.72px] font-['HarmonyOS_Sans_SC'] mb-2">
                                        SCORE
                                    </h3>
                                    <div className="space-y-2">
                                        <p className="text-[24px] font-bold text-[#333333] font-['HarmonyOS_Sans_SC']">
                                            8.2/10
                                        </p>
                                        <p className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">
                                            Scorecard as of June 16, 2025.
                                        </p>
                                    </div>
                                </div>

                                {/* Security Policy */}
                                <div>
                                    <h3 className="text-[18px] font-bold text-[#333333] tracking-[0.72px] font-['HarmonyOS_Sans_SC'] mb-2">
                                        Security Policy
                                    </h3>
                                    <div className="space-y-2">
                                        {[
                                            { name: 'Security-Policy', score: '10/10', expanded: true, details: 'Found 28/30 approved changesets -- score normalized to 9' },
                                            { name: 'Code-Review', score: '10/10' },
                                            { name: 'Maintained', score: '10/10' },
                                            { name: 'CI/Best-Practices', score: '10/10' },
                                            { name: 'License', score: '10/10' },
                                            { name: 'Dangerous-Workflow', score: '10/10' },
                                            { name: 'Token-Permissions', score: '10/10' },
                                            { name: 'Binary-Artifacts', score: '10/10' },
                                            { name: 'Pinned-Dependencies', score: '10/10' }
                                        ].map((item) => (
                                            <div key={item.name} className="flex items-center gap-2">
                                                <svg className="w-4 h-4 text-green-500" fill="currentColor" viewBox="0 0 20 20">
                                                    <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                                                </svg>
                                                <span className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">{item.name}</span>
                                                <span className="text-[14px] text-[#333333] font-['HarmonyOS_Sans_SC'] font-normal">{item.score}</span>
                                                {item.expanded && (
                                                    <p className="text-[12px] text-[#666666] font-['HarmonyOS_Sans_SC'] font-normal ml-6">
                                                        {item.details}
                                                    </p>
                                                )}
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
CratePage.getProviders = (page: any, pageProps: any) => {
    return (
        <AuthAppProviders {...pageProps}>
            <AppLayout {...pageProps}>{page}</AppLayout>
        </AuthAppProviders>
    );
};

export default CratePage; 