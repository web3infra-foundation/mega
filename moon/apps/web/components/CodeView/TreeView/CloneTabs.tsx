import React, { useEffect, useState } from 'react'
import { Box, Flex, Tabs } from '@radix-ui/themes'
import copy from 'copy-to-clipboard'
import { usePathname } from 'next/navigation'

import { LEGACY_API_URL } from '@gitmono/config'
import { Button, cn, Popover, PopoverContent, PopoverPortal, PopoverTrigger } from '@gitmono/ui'
import { CheckIcon, CopyIcon, DownloadIcon } from '@gitmono/ui/Icons'

const CloneTabs = ({ endpoint }: any) => {
  const pathname = usePathname()
  const [text, setText] = useState<string>(pathname || '')
  const [copied, setCopied] = useState<boolean>(false)
  const [active_tab, setActiveTab] = useState<string>('HTTP')
  const [open, setOpen] = useState(false)
  const url = new URL(LEGACY_API_URL)

  useEffect(() => {
    if (LEGACY_API_URL) {
      const url = new URL(LEGACY_API_URL)

      if (active_tab === '1') {
        setText(`${url.href}${pathname?.replace('/myorganization/code/tree/', '')}.git`)
      } else {
        setText(`ssh://git@${url.host}${pathname?.replace('/myorganization/code/tree', '')}.git`)
      }
    }
  }, [pathname, active_tab, endpoint])

  const handleCopy = () => {
    copy(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000) // Reset after 2 seconds
  }

  const tabContent = [
    {
      value: 'HTTP',
      inputValue: `${url.href}${pathname?.replace('/myorganization/code/tree/', '')}.git`,
      info: 'Clone using the web URL.'
    },
    {
      value: 'SSH',
      inputValue: `ssh://git@${url.host}${pathname?.replace('/myorganization/code/tree', '')}.git`,
      info: 'Use a password-protected SSH key.'
    }
  ]

  return (
    <>
      <Popover open={open} onOpenChange={setOpen} modal>
        <PopoverTrigger asChild>
          <Button variant='base'>
            <Flex gap='3'>
              <DownloadIcon />
              Code
            </Flex>
          </Button>
        </PopoverTrigger>
        <PopoverPortal>
          {open && (
            <PopoverContent
              className={cn(
                'animate-scale-fade shadow-popover dark:border-primary-opaque bg-primary relative flex h-[130px] w-[400px] flex-1 origin-[--radix-hover-card-content-transform-origin] flex-col overflow-hidden rounded-lg border border-transparent dark:shadow-[0px_2px_16px_rgba(0,0,0,1)]'
              )}
              sideOffset={4}
              align='start'
              onOpenAutoFocus={(e) => e.preventDefault()}
              asChild
              addDismissibleLayer
            >
              <Tabs.Root defaultValue='HTTP' onValueChange={setActiveTab}>
                <Tabs.List size='1' className='p-2'>
                  <Tabs.Trigger value='HTTP'>
                    <Button variant={active_tab === 'HTTP' ? 'flat' : 'plain'}>HTTP</Button>
                  </Tabs.Trigger>
                  <Tabs.Trigger value='SSH' style={{ marginLeft: '10px' }}>
                    <Button variant={active_tab === 'SSH' ? 'flat' : 'plain'}>SSH</Button>
                  </Tabs.Trigger>
                </Tabs.List>
                <Box pt='3'>
                  {tabContent?.map((_item) => {
                    return (
                      <Tabs.Content value={_item.value} key={_item.value}>
                        <Flex direction='column'>
                          <Flex align='center'>
                            <input
                              value={_item.inputValue}
                              className='bg-gray-150 m-2 w-[350px] border-b border-r p-1'
                              style={{ borderRadius: '5px' }}
                            />
                            <Button onClick={handleCopy} size='sm' variant='text' className='text-gray-600'>
                              {copied ? <CheckIcon /> : <CopyIcon />}
                            </Button>
                          </Flex>
                          <div className='ml-2 text-gray-500'>{_item.info}</div>
                        </Flex>
                      </Tabs.Content>
                    )
                  })}
                </Box>
              </Tabs.Root>
            </PopoverContent>
          )}
        </PopoverPortal>
      </Popover>
    </>
  )
}

export default CloneTabs
