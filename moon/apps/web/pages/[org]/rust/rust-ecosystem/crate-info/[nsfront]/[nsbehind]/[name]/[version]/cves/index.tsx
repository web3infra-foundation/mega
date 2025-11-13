"use client";
import React, { useEffect, useState } from 'react';
import Head from 'next/head';
import { useParams } from 'next/navigation';
import { useRouter } from 'next/router';
import Image from 'next/image';
import { AppLayout } from '@/components/Layout/AppLayout';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import CrateInfoLayout from '../layout';

interface CVE {
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
}

interface CratesInfo {
    crate_name: string;
    description: string;
    cves: CVE[];
    dep_cves: CVE[];
    versions: string[];
}

export default function CvesPage() {
    const params = useParams();
    const router = useRouter();
    const [expandedIdx, setExpandedIdx] = useState<number | null>(null);
    const [cveList, setCveList] = useState<CVE[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    // 从URL参数中获取crate信息
    const crateName = (params?.name as string) || 'example-crate';
    const version = (params?.version as string) || '1.0.0';
    const nsfront = (params?.nsfront as string) || (router.query.org as string);
    const nsbehind = (params?.nsbehind as string) || 'rust/rust-ecosystem/crate-info';

    // 获取crate信息（包含cves和dep_cves）
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

                const data: CratesInfo = await response.json();
                
                // 合并 cves 和 dep_cves 数组
                const allCves = [...(data.cves || []), ...(data.dep_cves || [])];

                setCveList(allCves);
            } catch (err) {
                setError('Failed to load CVE information');
                setCveList([]);
            } finally {
                setLoading(false);
            }
        };

        if (crateName && version && nsfront && nsbehind) {
            fetchCrateInfo();
        }
    }, [crateName, version, nsfront, nsbehind]);

    return (
        <>
            <Head>
                <title>CVEs - {crateName}</title>
            </Head>
            <CrateInfoLayout>
                <div className="w-full">
                    {/* All CVEs 标题 */}
                    <div className="w-full mt-4">
                        <div style={{
                            width: '100%',
                            paddingLeft: 'max(20px, 2vw)',
                            paddingRight: 'max(32px, 4vw)'
                        }}>
                            <h1
                                style={{
                                    display: '-webkit-box',
                                    WebkitBoxOrient: 'vertical',
                                    WebkitLineClamp: 1,
                                    overflow: 'hidden',
                                    color: '#1c2024',
                                    textOverflow: 'ellipsis',
                                    fontFamily: 'HarmonyOS Sans SC',
                                    fontSize: '24px',
                                    fontStyle: 'normal',
                                    fontWeight: 700,
                                    lineHeight: '24px',
                                    letterSpacing: 'var(--Typography-Letter-spacing-3, 0)',
                                }}
                            >
                                All CVEs
                            </h1>
                        </div>
                    </div>

                    {/* CVE 信息区 */}
                    <div className="w-full mt-4">
                        <div style={{
                            width: '100%',
                            paddingLeft: 'max(20px, 2vw)',
                            paddingRight: 'max(32px, 4vw)'
                        }}>
                            {/* 标题下方的分割线 */}
                            <div
                                style={{
                                    borderBottom: '1px solid #E5E7EB',
                                    width: '100%',
                                    height: 0,
                                    marginBottom: 0,
                                }}
                            />
                            
                            {/* 加载状态 */}
                            {loading && (
                                <div className="flex justify-center items-center py-8">
                                    <div className="text-gray-500">加载中...</div>
                                </div>
                            )}

                            {/* 错误状态 */}
                            {error && (
                                <div className="flex justify-center items-center py-8">
                                    <div className="text-red-500">{error}</div>
                                </div>
                            )}

                            {/* CVE列表 */}
                            {!loading && !error && (
                                <div className="space-y-0">
                                    {cveList.length > 0 ? (
                                        cveList.map((item, idx) => (
                                            <div key={item.id} style={{ position: 'relative' }}>
                                                <div
                                                    className="flex flex-col md:flex-row md:items-center justify-between py-4 min-h-[51px] md:min-h-[51px] md:items-center"
                                                >
                                                    <div className="flex flex-col md:flex-row md:items-center gap-2 mt-0 flex-1">
                                                        <span
                                                            className="font-medium text-lg text-gray-900 cursor-pointer hover:text-blue-600"
                                                            style={{
                                                                display: '-webkit-box',
                                                                WebkitBoxOrient: 'vertical',
                                                                WebkitLineClamp: 1,
                                                                overflow: 'hidden',
                                                                color: '#1c2024',
                                                                textOverflow: 'ellipsis',
                                                                fontFamily: 'HarmonyOS Sans SC',
                                                                fontSize: '18px',
                                                                fontStyle: 'normal',
                                                                fontWeight: 400,
                                                                lineHeight: '24px',
                                                                letterSpacing: 'var(--Typography-Letter-spacing-3, 0)',
                                                            }}
                                                            onClick={() => router.push(`/${router.query.org}/rust/rust-ecosystem/ecosystem-cve/cve-info?cveId=${item.id}`)}
                                                        >
                                                            {item.id}
                                                        </span>
                                                    </div>
                                                    <button
                                                        className="ml-4 flex items-center justify-center w-8 h-8"
                                                        onClick={() => setExpandedIdx(expandedIdx === idx ? null : idx)}
                                                        style={{ outline: 'none', border: 'none', background: 'transparent', cursor: 'pointer' }}
                                                    >
                                                        <Image
                                                            src="/rust/rust-ecosystem/down.png"
                                                            alt="toggle"
                                                            width={20}
                                                            height={20}
                                                            style={{ transition: 'transform 0.2s', transform: expandedIdx === idx ? 'rotate(180deg)' : 'none' }}
                                                        />
                                                    </button>
                                                </div>
                                                {expandedIdx === idx && (
                                                    <div className="pb-4">
                                                        <div className="text-gray-500 text-sm mb-2">
                                                            <div className="mb-2">
                                                                <strong>Description: </strong>{item.description}
                                                            </div>
                                                            <div className="mb-2">
                                                                <strong>Subtitle: </strong>{item.subtitle}
                                                            </div>
                                                        </div>
                                                        <div className="flex justify-end">
                                                            <button
                                                                style={{
                                                                    display: 'inline-flex',
                                                                    height: '24px',
                                                                    padding: '0 8px',
                                                                    justifyContent: 'center',
                                                                    alignItems: 'center',
                                                                    gap: '4px',
                                                                    flexShrink: 0,
                                                                    borderRadius: '3px',
                                                                    background: '#0047f112',
                                                                    color: '#3E63DD',
                                                                    fontWeight: 500,
                                                                    fontSize: '14px',
                                                                    border: 'none',
                                                                    outline: 'none',
                                                                    cursor: 'pointer',
                                                                }}
                                                                onClick={() => router.push(`/${router.query.org}/rust/rust-ecosystem/ecosystem-cve/cve-info?cveId=${item.id}`)}
                                                            >
                                                                Details
                                                            </button>
                                                        </div>
                                                    </div>
                                                )}
                                                {/* 分割线 */}
                                                {idx !== cveList.length - 1 && (
                                                    <div
                                                        style={{
                                                            borderBottom: '1px solid #E5E7EB',
                                                            width: '100%',
                                                            height: 0,
                                                        }}
                                                    />
                                                )}
                                            </div>
                                        ))
                                    ) : (
                                        <div className="flex justify-center items-center py-8">
                                            <div className="text-gray-500">暂无 CVE 信息</div>
                                        </div>
                                    )}
                                    {/* 最后一个CVE下方的分割线 */}
                                    {cveList.length > 0 && (
                                        <div
                                            style={{
                                                borderBottom: '1px solid #E5E7EB',
                                                width: '100%',
                                                height: 0,
                                            }}
                                        />
                                    )}
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            </CrateInfoLayout>
        </>
    );
}

CvesPage.getProviders = (page: any, pageProps: any) => {
    return (
        <AuthAppProviders {...pageProps}>
            <AppLayout {...pageProps}>{page}</AppLayout>
        </AuthAppProviders>
    );
};

