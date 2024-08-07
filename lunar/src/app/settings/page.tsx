'use client'

import { Divider } from '@/components/catalyst/divider'
import { Heading, Subheading } from '@/components/catalyst/heading'
import { Input } from '@/components/catalyst/input'
import { Text } from '@/components/catalyst/text'
import { invoke } from '@tauri-apps/api/tauri'
import { useState } from 'react'
import { Button } from "antd";

interface MegaStartParams {
  bootstrap_node: string,
}

export default function Settings() {

  const [loadings, setLoadings] = useState<boolean[]>([]);
  const [params, setParams] = useState<MegaStartParams>({
    bootstrap_node: "",
  });

  const enterLoading = (index: number) => {
    setLoadings((prevLoadings) => {
      const newLoadings = [...prevLoadings];
      newLoadings[index] = true;
      return newLoadings;
    });
    setTimeout(() => {
      setLoadings((prevLoadings) => {
        const newLoadings = [...prevLoadings];
        newLoadings[index] = false;
        return newLoadings;
      });
    }, 6000);
  }

  const startMega = async () => {
    try {
      await invoke('start_mega_service', { params: { params } });
      console.log('Mega service started successfully');
    } catch (error) {
      console.error('Failed to start Mega service', error);
    }
  };

  const stopMega = async () => {
    try {
      await invoke('stop_mega_service');
      console.log('Mega service stopped successfully');
    } catch (error) {
      console.error('Failed to stop Mega service', error);
    }
  };

  const restartMega = async () => {
    enterLoading(1);
    try {
      await invoke('restart_mega_service', { params: params });
      console.log('Mega service restarted successfully');
    } catch (error) {
      console.error('Failed to restart Mega service', error);
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setParams((prevParams) => ({
      ...prevParams,
      [name]: value,
    }));
  };

  return (
    <form method="post" className="mx-auto max-w-4xl">
      <Heading>Settings</Heading>
      <Divider className="my-10 mt-6" />

      <section className="grid gap-x-8 gap-y-6 sm:grid-cols-2">
        <div className="space-y-1">
          <Subheading>ZTM Server IP Address</Subheading>
          <Text>This will be restart Mega server.</Text>
        </div>
        <div>
          <Input disabled={loadings[1]} aria-label="Bootstrap Node" name="bootstrap_node"
            value={params.bootstrap_node}
            onChange={handleInputChange}
            placeholder="http://34.84.172.121/relay" />
        </div>
      </section>

      <Divider className="my-10" soft />

      <div className="flex justify-end gap-4">
        <Button>
          Reset
        </Button>
        <Button type="primary" loading={loadings[1]} onClick={restartMega} >Save changes</Button>
      </div>
    </form>
  )
}
