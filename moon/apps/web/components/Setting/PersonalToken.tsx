import React, {useState} from 'react'
import {LoadingSpinner, LockIcon, Button, PlusIcon} from '@gitmono/ui'
import {useGetTokenList} from '@/hooks/useGetTokenList'
import {usePostTokenGenerate} from '@/hooks/usePostTokenGenerate'
import {useDeleteTokenById} from '@/hooks/useDeleteTokenById'
import {useQueryClient} from '@tanstack/react-query'
import {legacyApiClient} from '@/utils/queryClient'
import {ListToken} from '@gitmono/types'
import toast from "react-hot-toast";
import HandleTime from "@/components/MrView/components/HandleTime";
import {useGetCurrentUser} from "@/hooks/useGetCurrentUser";

const TokenItem = ({item}: {
  item: ListToken
}) => {
  const {mutate: deleteToken} = useDeleteTokenById()
  const queryClient = useQueryClient()
  const fetchTokenList = legacyApiClient.v1.getApiUserTokenList()

  return (
    <div className="flex items-center justify-between py-4 border-b border-gray-200 last:border-b-0">
      <div className="flex items-start">
        <LockIcon className="w-6 h-6 text-gray-400" aria-hidden="true"/>
        <div className="ml-4">
          <p className="text-base font-bold text-gray-900">Token #{item.id}</p>
          <p className="text-sm font-mono text-gray-500 mt-1 break-all">{item.token}</p>
          <p className="text-xs text-gray-500 mt-2">
            <HandleTime created_at={item.created_at}/>
          </p>
        </div>
      </div>
      <button
        onClick={() =>
          deleteToken(
            {keyId: item.id},
            {
              onSuccess: () => {
                queryClient.invalidateQueries({queryKey: fetchTokenList.requestKey()})
              }
            }
          )
        }
        className="px-4 py-1 text-sm font-semibold text-red-500 border border-gray-300 rounded-md hover:bg-red-500 hover:text-white transition-colors duration-200"
      >
        Delete
      </button>
    </div>
  )
}

const CopySpace = ({ copyText }: { copyText: string }) => {
  const [copied, setCopied] = useState(false)
  const handleCopy = async (copyText: string) => {
    if (navigator.clipboard) {
      await navigator.clipboard
        .writeText(copyText)
        .then(() => toast.success("Copied to clipboard"))
        .catch(() => toast.error("Copied failed"))
    } else {
      const textArea = document
        .createElement('textarea')

      textArea.value = copyText
      document.body.appendChild(textArea)
      textArea.select()
      try {
        document.execCommand('copy')
        toast.success('Copied to clipboard')
        document.body.removeChild(textArea)
      } catch {
        toast.error("Copied failed")
      }
    }
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="mb-4">
      <code
        className="px-3 py-2 bg-white border border-gray-200 rounded font-mono text-sm break-all flex-1"
      >
        {copyText}
      </code>
      <Button variant="flat" className="ml-3" onClick={()=> handleCopy(copyText)}>
        {copied ? 'Copied' : 'Copy'}
      </Button>
    </div>
  )
}

const PersonalToken = () => {
  const {tokenList, isLoading} = useGetTokenList()
  const {mutate: generateToken, isPending: isGenerating} = usePostTokenGenerate()
  const queryClient = useQueryClient()
  const fetchTokenList = legacyApiClient.v1.getApiUserTokenList()

  const [generated, setGenerated] = useState<string | null>(null)
  const { data: currentUser, isLoading: isUserLoading } = useGetCurrentUser()

  const handleGenerate = () => {
    generateToken(undefined, {
      onSuccess: (result) => {
        setGenerated(result?.data ?? null)
        queryClient.invalidateQueries({queryKey: fetchTokenList.requestKey()})
      }
    })
  }

  return (
    <div className="bg-white text-gray-700 p-8 rounded-lg border border-gray-200 max-w-4xl mx-auto font-sans mt-8">
      <header className="flex items-center justify-between pb-4">
        <h1 className="text-3xl font-bold text-gray-900">Personal tokens</h1>
        <Button
          variant="primary"
          leftSlot={<PlusIcon/>}
          onClick={handleGenerate}
          disabled={isGenerating}
          loading={isGenerating}
          className="bg-[#1f883d]"
        >
          New token
        </Button>
      </header>

      <p className="mb-8">
        This is a list of personal access tokens associated with your account. Remove any tokens that you do not
        recognize.
      </p>

      {generated && currentUser && (
        <div className="mb-8 p-4 border border-green-200 rounded-md bg-green-50">
          <p className="text-sm text-gray-700">Your new token has been generated.</p>
          <div className="mt-2 flex-col items-center">
            <span className="text-sm text-gray-700">Username:</span>
            <CopySpace copyText={currentUser.username} />
            <span className="text-sm text-gray-700">Token:</span>
            <CopySpace copyText={generated}/>
          </div>
          <p className="text-xs text-gray-500 mt-2">Make sure to copy your new token now. You won’t be able to see it
            again.</p>
        </div>
      )}

      <section>
        <h2 className="text-xl font-semibold text-gray-900 pb-2 border-b border-gray-200">Tokens</h2>
        {(isLoading || isUserLoading) ? (
          <div className="flex h-[400px] items-center justify-center">
            <LoadingSpinner/>
          </div>
        ) : (
          <div>
            {currentUser && tokenList.map((item) => (
              <TokenItem key={item.id} item={item}/>
            ))}
          </div>
        )}
      </section>
    </div>
  )
}

export default PersonalToken
