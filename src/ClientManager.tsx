import { useEffect, useState,
    useRef, useLayoutEffect } from 'react';
import { useParams }          from 'react-router';
import { invoke }             from '@tauri-apps/api/core';
import { ClientEntry,
    ServerEntry }             from './ClientTypes';
import {listen, UnlistenFn}   from "@tauri-apps/api/event";
import Auth                   from './Auth';
import { AnsiHtml }           from 'fancy-ansi/react'

// TODO refactor into actual modular types
type ChatEventPayload = {
    message: string
}

type ChatEvent = {
    event: string,
    id?: number,
    payload: { Chat: ChatEventPayload }
}

type BackendClientConnection = {
    id: string,
    server: ServerEntry,
    version: string
}

type Connection = {
    id: string,
    server: string,
    version: string,
    connected: boolean
}


export default function ClientManager() {
    const { id } = useParams<{ id: string }>();

    const [allowed, setAllowed] = useState<{value: boolean, error?: string}>({value: true});
    const [loading, setLoading] = useState(true);
    const [showAuth, setShowAuth] = useState(false);

    const [client, setClient] = useState<ClientEntry | null>(null);
    const [connections, setConnections] = useState<Connection[]>([]);
    const [expandedConnection, setExpandedConnection] = useState<number>(-1);

    const [chatHistory, setChatHistory] = useState<Record<string, string[]>>({});
    const [chatMessage, setChatMessage] = useState<string>("");

    const [versions, setVersions] = useState<string[]>([]);
    const [servers, setServers] = useState<string[]>([]);

    const [showNewDialog, setShowNewDialog] = useState(false);
    const [selectedServer, setSelectedServer] = useState<string>("");
    const [selectedVersion, setSelectedVersion] = useState<string>("");
    const [errLabel, setErrLabel] = useState<string | null>(null);

    const connectionListener = useRef<UnlistenFn | undefined>();

    const sendChatMessage = (connection: Connection, message: string) => {
        if (!allowed || !client) return;
        if (chatMessage.trim()) {
            invoke("send_chat", { id: client.id, key: connection.id, message })
                .then(_ => {
                    setChatMessage('');
                    setErrLabel('');
                })
                .catch(e => {
                    console.log(e);
                    setErrLabel(e);
                });
        }
    }

    useEffect(() => {
        invoke('get_servers')
            .then((servers) => {
                let entries = servers as ServerEntry[];
                setServers(entries.map(entry => entry.name));
            })
            .catch((error) => {
                console.error('Failed to fetch servers:', error);
            });

        invoke('get_available_versions')
            .then((versions) => {
                setVersions(versions as string[]);
            })
            .catch((error) => {
                console.error('Failed to fetch versions:', error);
            });

        if (id) {
            invoke('get_client', {id: id})
                .then((clientData) => {
                    setClient(clientData as ClientEntry);
                    setLoading(false);
                })
                .catch((error) => {
                    console.error('Failed to fetch client:', error);
                    setLoading(false);
                });
        }
    }, []);

    const pollStatus = async () => {
        if (!client) return;

        type PollResponse = Record<string, [boolean, BackendClientConnection]>;
        const poll: Promise<PollResponse> =
            invoke("get_instances", {id: client.id});
        poll.then((response: PollResponse) => {
            setConnections(Object.entries(response)
                .sort(([_key1, [_state1, conn1]],
                       [_key2, [_state2, conn2]]) => {
                    const version1Parts = conn1.version.split('.').map(Number);
                    const version2Parts = conn2.version.split('.').map(Number);

                    for (let i = 0; i < Math.max(version1Parts.length, version2Parts.length); i++) {
                        const v1 = version1Parts[i] || 0;
                        const v2 = version2Parts[i] || 0;
                        if (v1 !== v2) {
                            return v2 - v1;
                        }
                    }
                    return conn2.server.name.length - conn1.server.name.length;
                }).map(
                ([_key, [state, connection]]) => {
                    return {
                        id: connection.id,
                        server: connection.server.name,
                        version: connection.version,
                        connected: state
                    } as Connection;
                }
            ))
        }).catch((e) => {
            console.log('Failed to fetch status:', e);
        });
    };

    const intervalRef = useRef(0);
    useLayoutEffect(() => {
        invoke("recall_authentication", { id: id })
            .then(b => {
                setAllowed({value: b as boolean});
                setLoading(false);
            })
            .catch(e => {
                console.error('Authentication error:', e);
                setAllowed({value: false, error: e});
                setLoading(false);
            })

        pollStatus();
        intervalRef.current = setInterval(pollStatus, 5000);
        return () => clearInterval(intervalRef.current);
    }, [client]);

    const chatContainerRef = useRef(null);
    useEffect(() => {
        const container = chatContainerRef.current as HTMLDivElement | null;
        if (container?.scrollHeight) {
            container.scrollTop = container.scrollHeight;
        }
    }, [chatHistory]);

    if (loading) {
        return <div className="p-4">Loading...</div>;
    }
    if (!client) {
        return <div className="p-4">Client not found</div>;
    }

    return (
        <main className="p-4">
            {allowed.value ?
            <div className="bg-slate-800 rounded-lg p-6">
                <div className="flex items-center space-x-4">
                    <img
                        src={`https://mc-heads.net/head/${client.uuid}/256`}
                        alt="Player head"
                        className="bg-slate-700 p-3 rounded-2xl h-28 w-28"
                    />
                    <div>
                        <h1 className="text-2xl font-bold">{client.username}</h1>
                        <p className="text-gray-400">
                            {client.auth ? 'Microsoft Account' : 'Offline Account'}
                        </p>
                        <p className="text-xs md:text-sm text-gray-500">
                            KEY: {client.id}
                        </p>
                        <p className="text-xs md:text-sm text-gray-500">
                            UUID: {client.uuid || 'Not available'}
                        </p>
                    </div>
                    <button
                        className="ml-auto bg-fuchsia-600 hover:bg-fuchsia-700 font-bold py-2 px-4 rounded
                                 transition duration-400 ease-in-out transform hover:scale-104"
                        onClick={() => setShowNewDialog(true)}
                    >
                        <i className="fa fa-plus mr-2 mt-2"/>
                        Instance
                    </button>
                    {showNewDialog && (
                        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center">
                            <div className="bg-slate-800 p-6 w-1/2 rounded-lg">
                                <h2 className="text-xl font-bold mb-4">New Instance</h2>
                                <div className="space-y-4">
                                    <div>
                                        <label className="block mb-2">Version</label>
                                        <select
                                            className="w-full appearance-none bg-slate-700 rounded px-3 py-2"
                                            value={selectedVersion}
                                            onChange={(e) => setSelectedVersion(e.target.value)}
                                        >
                                            <option value="">Select version...</option>
                                            {versions.map((version) => (
                                                <option key={version} value={version}>{version}</option>
                                            ))}
                                        </select>
                                    </div>
                                    <div>
                                        <label className="block mb-2">Server</label>
                                        <select
                                            className="w-full appearance-none bg-slate-700 rounded px-3 py-2"
                                            value={selectedServer}
                                            onChange={(e) => setSelectedServer(e.target.value)}
                                        >
                                            <option value="">Select server...</option>
                                            {servers.map((server) => (
                                                <option key={server} value={server}>{server}</option>
                                            ))}
                                        </select>
                                        {errLabel && (
                                            <p className="text-red-500 text-sm mt-2">{errLabel}</p>
                                        )}
                                    </div>
                                    <div className="flex justify-end space-x-2 mt-4">
                                        <button
                                            className="bg-gray-600 hover:bg-gray-700 duration-300 px-4 py-2 rounded"
                                            onClick={() => {
                                                setShowNewDialog(false);
                                                setSelectedVersion("");
                                                setSelectedServer("");
                                            }}
                                        >
                                            Cancel
                                        </button>
                                        <button
                                            className="bg-fuchsia-600 hover:bg-fuchsia-800 duration-300 px-4 py-2 rounded"
                                            onClick={() => {
                                                if (selectedVersion && selectedServer) {
                                                    const existingConnection = connections.find(
                                                        conn => conn.server === selectedServer && conn.version === selectedVersion
                                                    );
                                                    if (existingConnection) {
                                                        setErrLabel("Connection with this server and version already exists");
                                                        return;
                                                    }

                                                    invoke('create_connection', {
                                                        id: client.id,
                                                        serverName: selectedServer,
                                                        version: selectedVersion
                                                    }).then((id) => {
                                                        setShowNewDialog(false);
                                                        setSelectedVersion("");
                                                        setSelectedServer("");
                                                        console.log(`Create client instance: ${id}`);
                                                        pollStatus();
                                                    }).catch((error) => {
                                                        console.error('Failed to create connection:', error);
                                                    });
                                                }
                                            }}
                                            disabled={!selectedVersion || !selectedServer}
                                        >
                                            Create
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    )}
                </div>
                <hr className="my-6 border-slate-600"/>
                <div className="mt-6">
                    <h2 className="text-xl font-bold mb-4">{(connections.length == 0 ? 'No ' : '') + 'Instances'}</h2>
                    {!showNewDialog &&
                        connections.map((connection, index) => (
                        <div key={index} className={`mb-4 bg-slate-900 rounded-lg p-4
                                                        transition duration-150 
                                                    ${expandedConnection == index ? 'scale-102' : 'hover:scale-102'}`}>
                            <div
                                className="flex justify-between items-center cursor-pointer"
                                onClick={() => {
                                    if (connectionListener.current) connectionListener.current();

                                    let newConnection = expandedConnection == index ? -1 : index;
                                    setExpandedConnection(newConnection);

                                    if (newConnection != -1) {
                                        setErrLabel('');
                                        listen(connection.id, (e: ChatEvent) => {
                                            setChatHistory(
                                                (current) => ({
                                                    ...current,
                                                    [connection.id]: [
                                                        ...(current[connection.id] || []),
                                                        e.payload.Chat.message
                                                    ]
                                                })
                                            )
                                        }).then(unlisten => connectionListener.current = unlisten)
                                    }
                                }}
                            >
                                <div>
                                    <span className="font-bold">{connection.server}</span>
                                    <span className="text-gray-400 text-sm"> - {connection.version}</span>
                                    <span
                                        className={`mx-2 inline-block w-16 h-2 rounded-full ${
                                            connection.connected ? 'bg-green-400' : 'bg-red-400'
                                            }`}>
                                    </span>
                                    <span className="font-normal text-sm text-gray-500">id: {connection.id}</span>
                                </div>
                                <i className={`fa fa-chevron-${expandedConnection == index ? 'up' : 'down'}`}/>
                            </div>

                            {expandedConnection == index && (
                                <div className="mt-4">
                                    <div className="flex space-x-2 mb-4">
                                        <button
                                            className="bg-green-600 hover:bg-green-800 px-3 py-1 rounded
                                                        duration-300"
                                            disabled={connection.connected}
                                            onClick={async () => {
                                                try {
                                                    await invoke("connect_client", {id: client.id, key: connection.id});
                                                    setErrLabel(null)
                                                } catch (e) {
                                                    setErrLabel(e as string)
                                                }
                                            }}
                                        >
                                            Connect
                                        </button>
                                        <button
                                            className="bg-red-600 hover:bg-red-800 px-3 py-1 rounded
                                                        duration-300"
                                            disabled={!connection.connected}
                                            title="Gracefully shuts down client thread right after its handle is notified to disconnect, to ensure a smoother disconnection. Disconnection is near instant unless impacted by lag, but this is not guaranteed."
                                            onClick={async () => {
                                                try {
                                                    await invoke("kill_client_soft", {
                                                        id: client.id,
                                                        key: connection.id
                                                    });
                                                    setErrLabel(null);
                                                } catch (e) {
                                                    setErrLabel(e as string);
                                                }
                                            }}
                                        >
                                            Disconnect
                                        </button>
                                        <div className="relative inline-block">
                                            <button
                                                className="bg-red-800 hover:bg-red-950 px-3 py-1 rounded
                                                            duration-300"
                                                title="Directly and forcefully aborts client thread, instantly freeing resources.
Does not guarantee instant client disconnection, will likely be recognised as a crash by the server once it naturally times out."
                                                onClick={async () => {
                                                    try {
                                                        await invoke("kill_client", {
                                                            id: client.id,
                                                            key: connection.id
                                                        });
                                                        setErrLabel(null);
                                                    } catch (e) {
                                                        setErrLabel(e as string);
                                                    }
                                                }}
                                            >
                                                Kill
                                            </button>
                                        </div>
                                        {errLabel &&
                                            <div className="flex items-center">
                                                <span className="text-sm text-red-700">{errLabel}</span>
                                            </div>
                                        }
                                    </div>
                                    <div ref={chatContainerRef}
                                        className="bg-slate-800 rounded-lg p-4 h-96 overflow-y-auto mb-4">
                                        {(chatHistory[connection.id] || []).map((message, idx) => (
                                            <div>
                                                <AnsiHtml
                                                    key={idx}
                                                    className="text-sm font-normal"
                                                    text={message}
                                                />
                                            </div>
                                        ))}
                                    </div>
                                    <div className="flex space-x-2">
                                        <input
                                            type="text"
                                            className="flex-grow bg-slate-700 rounded px-3 py-2"
                                            placeholder="Type a message..."
                                            value={chatMessage}
                                            onChange={(e) => setChatMessage(e.target.value)}
                                            onKeyDown={(e) => {
                                                if (e.key === 'Enter' && chatMessage.trim()) {
                                                    sendChatMessage(connection, chatMessage);
                                                }
                                            }}
                                        />
                                        <button
                                            className="bg-blue-600 hover:bg-blue-800 px-4 py-2 rounded
                                                        duration-300"
                                            onClick={() => {
                                                if (chatMessage.trim()) {
                                                    sendChatMessage(connection, chatMessage);
                                                }
                                            }}
                                        >
                                            Send
                                        </button>
                                    </div>
                                </div>
                            )}
                        </div>
                    ))}
                </div>
            </div>
                :
                <div className="flex justify-center mt-12">
                    <div className="bg-red-800 rounded-lg text-red-50 w-1/2 p-8">
                        <p>This client has failed to authenticate.</p>
                        <p className="mt-2 text-sm text-red-300">
                            <span>Error details: </span>
                            <div dangerouslySetInnerHTML={{__html: allowed.error ? allowed.error : ''}}></div>
                            {!allowed.error && <span>Failed to authenticate client ${id}</span>}
                        </p>
                        <div className="flex justify-center">
                            <button
                                className="mt-4 bg-red-400 hover:bg-red-600 px-16 py-3 rounded duration-300"
                                onClick={() => setShowAuth(true)}
                            >
                                Re-authenticate
                            </button>
                        </div>
                    </div>
                </div>
            }
            {showAuth && (
                <Auth
                    onClose={() => setShowAuth(false)}
                    onFinish={() => {
                        setShowAuth(false);
                        window.location.reload();
                    }}
                />
            )}
        </main>
    );
}
