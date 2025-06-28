import {invoke}                     from "@tauri-apps/api/core";
import React, {useEffect, useState} from "react";

type ServerEntry = {
    name: string;
    ip: string;
    port: number;
    connections: number;
}

function get_server_ico(ip?: string): string {
    if (!ip) {
        return "https://mc-heads.net/helm/none/256";
    }
    return `https://eu.mc-api.net/v3/server/favicon/${ip}`;
}

function ServerList({servers, onRemove}: { servers: ServerEntry[], onRemove: (name: string) => void }) {
    return (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 p-4">
            {servers ? "" : "No servers registered"}
            {servers.map((server, index) => (
                <div key={index} className="flex items-center
                                    bg-slate-800 rounded-lg cursor-pointer
                                    hover:bg-slate-950 hover:scale-103
                                    hover:drop-shadow-lg hover:drop-shadow-fuchsia-600
                                    duration-400 ease-out">
                    <div className="p-4 min-w-10/12 h-max flex items-center space-x-4"
                         onClick={() => console.log(server.name)}>
                        <div className="min-w-16 min-h-16 max-w-16 max-h-16 bg-slate-700 rounded-lg
                                flex items-center justify-center">
                            <img src={get_server_ico(server.ip)} alt="server icon"/>
                        </div>
                        <div className="flex-1">
                            <h3 className="font-semibold">{server.name}</h3>
                            <div className="flex-col space-x-2 text-sm">
                                <p className="text-gray-400">
                                    {server.ip} {server.port}
                                </p>
                                <p className="text-blue-400">
                                    {server.connections ? server.connections : 0}
                                    {server.connections == 1 ? " client" : " clients"} connected
                                </p>
                            </div>
                        </div>
                    </div>
                    <div className="w-24 h-24  cursor-pointer
                            duration-400 ease-out opacity-0 hover:opacity-100
                            hover:scale-103 flex justify-center items-center"
                         onClick={
                             () => {
                                 invoke('delete_server', { name: server.name })
                                     .then(_ => onRemove(server.name))
                             }
                         }>
                        <i className="fa fa-trash fa-2x"/>
                    </div>
                </div>
            ))}
        </div>
    );
}

async function getServers(): Promise<ServerEntry[]> {
    return await invoke('get_servers')
}

export default function Servers() {
    const [showDialog, setShowDialog] = useState(false);
    const [serverInput, setServerInput] = useState({name: "", ip: "", port: 25565});
    const [servers, setServers] = useState<ServerEntry[]>([]);

    useEffect(() => {
        getServers()
            .then(e => setServers(e))
            .catch(e => console.log(e))
    }, []);

    const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
        event.preventDefault();
        console.log(serverInput);
        invoke('add_server', serverInput)
            .then(_ => {
                setServers(
                    [
                        ...servers,
                        {
                            name: serverInput.name,
                            ip: serverInput.ip,
                            port: serverInput.port,
                            connections: 0
                        }
                    ]
                )
            })
            .catch(e => console.log(e))
            .finally(() => setShowDialog(false))
    };

    return (
        <main className="p-4">
            <div className="flex justify-between items-center mb-4">
                <h1 className="text-2xl font-bold">Registered Servers</h1>
                <button onClick={() => setShowDialog(true)}
                        className="bg-fuchsia-600 hover:bg-fuchsia-700 font-bold py-2 px-4 rounded
                             transition duration-400 ease-in-out transform hover:scale-106 mx-4">
                    <i className="fa fa-plus"/>
                </button>
            </div>
            {servers && <ServerList
                servers={servers}
                onRemove={(name) => {
                    setServers(servers.filter(e => e.name !== name))
                }}
            />}
            {showDialog && (

                <div className="fixed inset-0 z-50 flex items-center justify-center">
                    <div className="fixed inset-0 bg-black bg-opacity-50" onClick={() => setShowDialog(false)}></div>
                    <div className="relative w-full max-w-md p-8 space-y-6 bg-slate-800 rounded-xl shadow-lg z-10">
                        <button
                            onClick={() => setShowDialog(false)}
                            className="absolute top-4 right-4 text-gray-400 hover:text-white"
                        >
                            ✕
                        </button>
                        <h2 className="text-2xl font-bold text-center">Add New Server</h2>
                        <form onSubmit={handleSubmit} className="space-y-4">
                            <input
                                type="text"
                                placeholder="Display Name"
                                className="w-full px-4 py-2 rounded bg-slate-700 text-white focus:outline-none focus:ring-2 focus:ring-fuchsia-500"
                                onChange={(e) => setServerInput({...serverInput, name: e.target.value})}
                            />
                            <input
                                type="text"
                                placeholder="IP Address"
                                className="w-full px-4 py-2 rounded bg-slate-700 text-white focus:outline-none focus:ring-2 focus:ring-fuchsia-500"
                                onChange={(e) => setServerInput({...serverInput, ip: e.target.value})}
                            />
                            <input
                                type="number"
                                placeholder="Port"
                                className="w-full px-4 py-2 rounded bg-slate-700 text-white focus:outline-none focus:ring-2 focus:ring-fuchsia-500"
                                defaultValue="25565"
                                onChange={(e) => setServerInput({...serverInput, port: parseInt(e.target.value)})}
                            />
                            <button
                                type="submit"
                                className="w-full px-4 py-2 text-white bg-fuchsia-600 rounded
                                hover:bg-fuchsia-800 focus:outline-none focus:ring-2
                                focus:ring-fuchsia-500 duration-400"
                            >
                                Register Server
                            </button>
                        </form>
                    </div>
                </div>
            )}
        </main>
    );
}