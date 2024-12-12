import { Card, Dropdown } from 'antd/lib';
import type { MenuProps } from 'antd';
import { MoreOutlined } from '@ant-design/icons';
import { formatDistance, fromUnixTime } from 'date-fns';
import LexicalContent from '@/components/rich-editor/LexicalContent';

const Comment = ({ conv, fetchDetail }) => {

    const delete_comment = async () => {
        await fetch(`/api/mr/comment/${conv.id}/delete`, {
            method: 'POST',
        });
    };
    const handleMenuClick: MenuProps['onClick'] = ({ key }) => {
        if (key === '3') {
            delete_comment()
            fetchDetail()
        }
    };

    const items: MenuProps['items'] = [
        {
            label: 'Edit',
            key: '1',
            disabled: true
        },
        {
            label: 'Hide',
            key: '2',
            disabled: true
        },
        {
            type: 'divider',
        },
        {
            label: 'Delete',
            key: '3',
            danger: true,
        }
    ];
    const menuProps = {
        items,
        onClick: handleMenuClick,
    };

    const time = formatDistance(fromUnixTime(conv.created_at), new Date(), { addSuffix: true });

    return (
        <Card size="small" title={"Mega commented " + time} style={{ border: "1px solid #d1d9e0" }} extra={
            <Dropdown menu={menuProps} trigger={['click']}>
                <MoreOutlined />
            </Dropdown>
        }>
            <LexicalContent lexicalJson={conv.comment} />
        </Card>
    )

}

export default Comment;