"use client";

import { useEffect, useState } from "react";

export function GetResult({ host }: { host: string }) {
    const [content, setData] = useState<React.ReactNode>(<p>Loading...</p>);
    useEffect(() => {
        fetch(host, {}).then(async (res) => {
            if (!res.ok) {
                // setData("error, can't get response:\n" + res);
                setData(
                    <div>
                        <h1>Get {host} faied</h1>
                        <pre>{res.text()}</pre>
                    </div>
                );
                return;
            }
            const data = await res.json();
            console.log(data);
            const content = (
                <div>
                    <h1>API Result of {host}</h1>
                    <text>{JSON.stringify(data, null, 0)}</text>
                </div>
            );
            setData(content);
        });
    }, [host]);
    return <div style={{ textAlign: "center" }}>{content}</div>;
}
