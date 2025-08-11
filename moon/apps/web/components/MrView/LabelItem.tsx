import {ConditionalWrap} from '@gitmono/ui'
import {useGetOrganizationMember} from '@/hooks/useGetOrganizationMember'
import {MemberHovercard} from '../InlinePost/MemberHovercard'
import {MemberAvatar} from '../MemberAvatar'
import {UserLinkByName} from './components/UserLinkByName'
import HandleTime from './components/HandleTime'
import {ConversationItem, GetApiLabelByIdData, type LabelItem} from '@gitmono/types/generated'
import {getFontColor} from "@/utils/getFontColor";
import {legacyApiClient} from "@/utils/queryClient";
import {useEffect, useMemo, useState} from "react";
import {apiErrorToast} from "@/utils/apiErrorToast";

interface LabelItemProps {
  conv: ConversationItem
}

function LabelItem({conv}: LabelItemProps) {
  const {data: member} = useGetOrganizationMember({username: conv.username})
  const comment = conv.comment?.split(' ') ?? []
  const match = useMemo(
    () =>
      conv.comment?.match(/\[(.*?)]/) ?? []
    , [conv.comment])
  const [idList, setIdList] = useState<string[]>([])
  const [labelList, setLabelList] = useState<LabelItem[]>([]);

  useEffect(() => {
    const res = (match.length > 1) ? match[1].split(", ") : []

    setIdList(res)
  }, [match])
  useEffect(() => {
    if (idList.length === 0) return;

    Promise.all(
      idList.map(id =>
        legacyApiClient.v1.getApiLabelById().request(parseInt(id, 10))
      )
    ).then((res: GetApiLabelByIdData[]) => {
      const fetchedLabels = res
        .filter(res => res?.data)
        .map(res => res.data);

      setLabelList(fetchedLabels);
    }).catch(err =>
      apiErrorToast(err)
    )
  }, [idList])

  return (
    <>
      <div className='flex flex-wrap items-center gap-2'>
        <ConditionalWrap
          condition={true}
          wrap={(c) => (
            <MemberHovercard username={conv?.username}>
              <UserLinkByName username={conv?.username} className='relative'>
                {c}
              </UserLinkByName>
            </MemberHovercard>
          )}
        >
          {member ? <MemberAvatar member={member} size='sm'/> : 'Avatar not found'}
        </ConditionalWrap>
        <div>
          <span className='font-semibold'>{conv.username} </span>
          <span className='text-gray-400'>{comment[1]} </span>
          {
            labelList.map(label => {
              const fontColor = getFontColor(label.color)

              if (!label) return null

              return <span
                key={label.id}
                style={{
                  backgroundColor: label.color,
                  color: fontColor.toHex(),
                  borderRadius: '16px',
                  padding: '0px 8px',
                  fontSize: '12px',
                  fontWeight: '550',
                  justifyContent: 'center',
                  textAlign: 'center'
                }}
              >
              {label.name}
            </span>
            })
          }
        </div>
        <div className='text-sm text-gray-500 hover:text-gray-700'>
          <HandleTime created_at={conv.created_at}/>
        </div>
      </div>
    </>
  )
}

export default LabelItem