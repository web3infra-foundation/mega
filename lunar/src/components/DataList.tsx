'use client'

import { Input, Modal, Space, Table, TableProps, Badge, Button } from 'antd/lib'
import { useEffect, useState } from 'react'
import { format, fromUnixTime } from 'date-fns'
import { DownloadOutlined } from '@ant-design/icons';


interface DataType {
    name: string;
    identifier: string;
    origin: string;
    update_time: number;
    commit: string;
    peer_online: boolean
}

const DataList = ({ data }) => {
    const [open, setOpen] = useState(false);
    const [confirmLoading, setConfirmLoading] = useState(false);
    const [inputPort, setInputPort] = useState("");
    const [isOkButtonDisabled, setIsOkButtonDisabled] = useState(true);
    const [modelRecord, setModalRecord] = useState<DataType>({
        name: "",
        identifier: "",
        origin: "",
        update_time: 0,
        commit: "",
        peer_online: true,
    });
    const [modal, contextHolder] = Modal.useModal();

    const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        // console.log("value changes", e.target.value);
        setInputPort(e.target.value);
        setIsOkButtonDisabled(e.target.value.length < 4);
    };

    const showSuccModel = (text) => {
        modal.success({
            title: 'Fork from remote success! Clone by command:',
            content: `${text}`,
        });
    };

    var columns: TableProps<DataType>['columns'] = [
        {
            title: 'Name',
            dataIndex: ['name', 'identifier'],
            key: 'name',
            render: (_, record) => {
                return <>
                    <Space>
                        <span>{record.name}</span>
                    </Space>
                </>
            }
        },
        {
            title: 'Identifier',
            dataIndex: 'identifier',
            key: 'identifier',
            ellipsis: true,
            render: (text) => <a>{text}</a>,
        },
        {
            title: 'Online',
            dataIndex: 'peer_online',
            key: 'peer_online',
            render: (_, { peer_online }) => (
                <Badge status={peer_online ? "success" : "default"} text={peer_online ? "On" : "Off"} />
            ),
        },
        {
            title: 'Origin',
            dataIndex: 'origin',
            key: 'origin',
        },
        {
            title: 'Update Date',
            dataIndex: 'update_time',
            key: 'update_time',
            render: (update_time) => (
                <Space>
                    <span>
                        {update_time && format(fromUnixTime(update_time), 'yyyy-MM-dd HH:mm:ss')}
                    </span>
                </Space>
            )
        },
        {
            title: 'Action',
            key: 'action',
            render: (_, record) => (
                <Space size="middle">
                    <Button disabled={!record.peer_online}
                        onClick={() => showModal(record)} type="primary" shape="round" icon={<DownloadOutlined />} size={'small'}>
                        Clone
                    </Button>
                </Space>
            ),
        },
    ];


    const showModal = (record) => {
        setModalRecord(record);
        setOpen(true);
    };

    const handleOk = () => {
        setConfirmLoading(true);

        const repoFork = async () => {
            try {
                let text = await getRepoFork(modelRecord.identifier, inputPort);
                setOpen(false);
                setConfirmLoading(false);
                showSuccModel(text);
            } catch (error) {
                console.error('Error fetching data:', error);
            }
        };
        repoFork();

        setTimeout(() => {
            setOpen(false);
            setConfirmLoading(false);
        }, 5000);
    };

    const handleCancel = () => {
        setOpen(false);
    };

    return (
        <div>
            <Table scroll={{ x: true }} columns={columns} dataSource={data} />
            <Modal
                title="Please input a local port"
                open={open}
                onOk={handleOk}
                confirmLoading={confirmLoading}
                onCancel={handleCancel}
                okButtonProps={{ disabled: isOkButtonDisabled }}
            >
                <Input showCount maxLength={5} value={inputPort} onChange={handleInputChange} />
            </Modal>
            {contextHolder}
        </div>
    );
};


async function getRepoFork(identifier, port) {
    const res = await fetch(`api/relay/repo_fork?identifier=${identifier}&port=${port}`);
    const response = await res.json();
    return response.data.data
}

export default DataList;
