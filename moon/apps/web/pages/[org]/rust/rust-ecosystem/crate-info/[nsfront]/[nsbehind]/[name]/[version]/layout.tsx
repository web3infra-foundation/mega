'use client'

import React, { useCallback, useEffect, useMemo, useState } from 'react'
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline'
import { useParams } from 'next/navigation'
import { useRouter } from 'next/router'

import { VersionSelectorDropdown } from '../../../../../../../../../components/Rust/VersionSelector/VersionSelectorDropdown'

interface CrateInfoLayoutProps {
  children: React.ReactNode
  versions?: string[]
}

const CrateInfoLayoutComponent = ({ children, versions = [] }: CrateInfoLayoutProps) => {
    const router = useRouter();
    const params = useParams();
    
    // 从URL参数中获取crate信息 - 使用更稳定的依赖项
    const crateName = useMemo(() => 
        params?.name as string || "example-crate", 
        [params?.name]
    );
    const version = useMemo(() => 
        params?.version as string || "1.0.0", 
        [params?.version]
    );
    const nsfront = useMemo(() => 
        params?.nsfront as string || router.query.org as string, 
        [params?.nsfront, router.query.org]
    );
    const nsbehind = useMemo(() => 
        params?.nsbehind as string || "rust/rust-ecosystem/crate-info", 
        [params?.nsbehind]
    );
    
    // 稳定的crate信息对象，避免不必要的重新渲染
    const crateInfo = useMemo(() => ({
        crateName,
        version,
        nsfront,
        nsbehind,
        org: router.query.org as string
    }), [crateName, version, nsfront, nsbehind, router.query.org]);
    
    // 版本选择相关状态
    const [isVersionDialogOpen, setIsVersionDialogOpen] = useState(false);
    const [selectedVersion, setSelectedVersion] = useState<string>(version);
    
    // 当version参数变化时更新selectedVersion
    useEffect(() => {
        setSelectedVersion(version);
    }, [version]);
    
    // 搜索相关状态
    const [searchQuery, setSearchQuery] = useState('');
    
    // 搜索处理函数
    const handleSearch = useCallback((e: React.FormEvent) => {
        e.preventDefault();
        if (searchQuery.trim()) {
            router.push({
                pathname: `/${crateInfo.org}/rust/rust-ecosystem/search`,
                query: { q: searchQuery.trim() }
            });
        }
    }, [searchQuery, router, crateInfo.org]);
    
    // 根据当前路径确定activeTab
    const [activeTab, setActiveTab] = useState<'overview' | 'dependencies' | 'dependents' | 'compare' | 'versions' | 'cves'>('overview');
    
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
        } else if (path.includes('/cves')) {
            setActiveTab('cves');
        } else {
            setActiveTab('overview');
        }
    }, [router.asPath]);

    const handleTabClick = useCallback((href: string) => {
        router.push(href, undefined, { shallow: true });
    }, [router]);

    // 版本选择处理函数
    const handleVersionSelect = useCallback((newVersion: string) => {
        if (newVersion === selectedVersion) return; // 如果版本相同，不执行任何操作
        
        setSelectedVersion(newVersion);
        // 更新URL中的版本参数
        const currentPath = router.asPath;
        
        const newPath = currentPath.replace(/\/[^/]+\/?$/, `/${newVersion}`);

        router.push(newPath, undefined, { shallow: true });
    }, [router, selectedVersion]);

    const navigationTabs = useMemo(() => [
        { id: 'overview', label: 'overview', href: `/${crateInfo.org}/rust/rust-ecosystem/crate-info/${crateInfo.nsfront}/${crateInfo.nsbehind}/${crateInfo.crateName}/${crateInfo.version}` },
        { id: 'dependencies', label: 'dependencies', href: `/${crateInfo.org}/rust/rust-ecosystem/crate-info/${crateInfo.nsfront}/${crateInfo.nsbehind}/${crateInfo.crateName}/${crateInfo.version}/dependencies` },
        { id: 'dependents', label: 'dependents', href: `/${crateInfo.org}/rust/rust-ecosystem/crate-info/${crateInfo.nsfront}/${crateInfo.nsbehind}/${crateInfo.crateName}/${crateInfo.version}/dependents` },
        { id: 'compare', label: 'compare', href: `/${crateInfo.org}/rust/rust-ecosystem/crate-info/${crateInfo.nsfront}/${crateInfo.nsbehind}/${crateInfo.crateName}/${crateInfo.version}/compare` },
        { id: 'versions', label: 'versions', href: `/${crateInfo.org}/rust/rust-ecosystem/crate-info/${crateInfo.nsfront}/${crateInfo.nsbehind}/${crateInfo.crateName}/${crateInfo.version}/versions` },
        { id: 'cves', label: 'cves', href: `/${crateInfo.org}/rust/rust-ecosystem/crate-info/${crateInfo.nsfront}/${crateInfo.nsbehind}/${crateInfo.crateName}/${crateInfo.version}/cves` }
    ], [crateInfo]);

  return (
    <div className='flex h-screen flex-col bg-[#F4F4F5]'>
      {/* 搜索栏 - 固定在顶部 */}
      <div className='flex w-full flex-shrink-0 justify-center' style={{ background: '#FFF' }}>
        <div
          className='flex items-center'
          style={{
            width: '1680px',
            height: '53px',
            flexShrink: 0,
            marginTop: 0,
            marginBottom: 0,
            paddingLeft: 32,
            paddingRight: 32,
            background: '#FFF',
            boxSizing: 'border-box'
          }}
        >
          <form onSubmit={handleSearch} className='ml-8 max-w-xl flex-1'>
            <div className='relative ml-10 mt-2'>
              <div className='pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3'>
                <MagnifyingGlassIcon className='h-5 w-5 text-gray-400' />
              </div>
              <input
                type='text'
                placeholder='Search...'
                className='block w-full border-0 bg-transparent py-2 pl-10 pr-3 text-gray-900 placeholder-gray-500 focus:outline-none focus:ring-0'
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => {
                  // 阻止可能导致页面刷新的键盘事件
                  if (e.key === 'Backspace' || e.key === 'Delete') {
                    e.stopPropagation()
                    return
                  }
                  // 阻止 Enter 键在输入框中的默认行为，让表单处理
                  if (e.key === 'Enter') {
                    e.preventDefault()
                    handleSearch(e)
                  }
                }}
                onKeyUp={(e) => {
                  // 确保键盘事件不会冒泡
                  e.stopPropagation()
                }}
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
            boxSizing: 'border-box'
          }}
        />
      </div>

      {/* 分类标签和版本选择区域 - 固定在搜索栏下方 */}
      <div className='flex w-full flex-shrink-0 justify-center' style={{ background: '#FFF' }}>
        <div style={{ width: '1370px', paddingLeft: 32, paddingRight: 32, paddingTop: 24 }}>
          {/* Crate信息 */}
          <div className='mb-6 flex items-center justify-between'>
            <div className='flex items-center space-x-4'>
              <div className='flex flex-col space-y-2'>
                <div className='text-sm text-gray-500'>Cargo crate</div>
                <h1
                  className='text-3xl font-bold text-gray-900'
                  style={{
                    color: '#1c2024',
                    fontFamily: '"HarmonyOS Sans SC"',
                    fontSize: '36px',
                    fontStyle: 'normal',
                    fontWeight: 400,
                    lineHeight: 'normal'
                  }}
                >
                  {crateInfo.crateName}
                </h1>
              </div>
              <div className='relative'>
                <button
                  onClick={() => setIsVersionDialogOpen(!isVersionDialogOpen)}
                  className='mt-8 flex items-center space-x-2 transition-colors hover:bg-gray-50'
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
                  <div className='flex h-6 w-6 items-center justify-center rounded-full border-2 border-gray-400 bg-transparent'>
                    <svg className='h-3 w-3 text-gray-400' fill='currentColor' viewBox='0 0 20 20'>
                      <path
                        fillRule='evenodd'
                        d='M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z'
                        clipRule='evenodd'
                      />
                    </svg>
                  </div>
                  <span className='text-lg font-medium text-gray-900'>{selectedVersion}</span>
                  <svg className='h-4 w-4 text-gray-500' fill='none' stroke='currentColor' viewBox='0 0 24 24'>
                    <path strokeLinecap='round' strokeLinejoin='round' strokeWidth={2} d='M19 9l-7 7-7-7' />
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
          <div className='mb-0 flex space-x-8'>
            {navigationTabs.map((tab) =>
              tab.href ? (
                <button
                  key={tab.id}
                  onClick={() => handleTabClick(tab.href!)}
                  className={`border-b-2 px-1 py-2 transition-colors ${
                    tab.id === activeTab
                      ? 'border-blue-500'
                      : 'border-transparent hover:border-gray-300 hover:text-gray-700'
                  }`}
                  style={{
                    color: tab.id === activeTab ? '#1c2024' : '#6b7280',
                    fontFamily: '"HarmonyOS Sans SC"',
                    fontSize: '16px',
                    fontStyle: 'normal',
                    fontWeight: 500,
                    lineHeight: '20px',
                    letterSpacing: '0'
                  }}
                >
                  {tab.label}
                </button>
              ) : (
                <button
                  key={tab.id}
                  className={`border-b-2 px-1 py-2 transition-colors ${
                    tab.id === activeTab
                      ? 'border-blue-500'
                      : 'border-transparent hover:border-gray-300 hover:text-gray-700'
                  }`}
                  style={{
                    color: tab.id === activeTab ? '#1c2024' : '#6b7280',
                    fontFamily: '"HarmonyOS Sans SC"',
                    fontSize: '16px',
                    fontStyle: 'normal',
                    fontWeight: 500,
                    lineHeight: '20px',
                    letterSpacing: '0'
                  }}
                >
                  {tab.label}
                </button>
              )
            )}
          </div>
        </div>
      </div>

      {/* 可滚动内容区域 */}
      <div className='flex-1 overflow-auto' style={{ background: '#F4F4F5' }}>
        {children}
      </div>
    </div>
  )
}

const CrateInfoLayout = React.memo(CrateInfoLayoutComponent)

CrateInfoLayout.displayName = 'CrateInfoLayout'

export default CrateInfoLayout
