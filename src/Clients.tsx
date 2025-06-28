import { ClientEntry }         from "./ClientTypes.tsx";
import { useNavigate }         from "react-router";
import { invoke }              from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import Auth                    from "./Auth.tsx";

export function ClientList({clients}: { clients: ClientEntry[] }) {
    const navigate = useNavigate();
    return (
        <main>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 p-4">
                {clients.map((client, index) => (
                    <div key={index}
                         onClick={() => {
                             navigate(`/client/${client.id}`)
                         }}
                         className="flex items-center space-x-4
                                    bg-slate-800 p-4 rounded-lg cursor-pointer
                                    hover:bg-slate-950 hover:scale-103
                                    hover:drop-shadow-lg hover:drop-shadow-fuchsia-600
                                    duration-400 ease-out">
                        <div className="w-16 h-16 bg-slate-700 rounded-lg
                                flex items-center justify-center">
                            <img src={`https://mc-heads.net/head/${client.uuid}/256`} alt="client head" />
                        </div>
                        <div className="flex-1">
                            <h3 className="font-semibold">{client.username}</h3>
                            <div className="flex-col space-x-2 text-sm">
                                <p className="text-gray-400">
                                    {client.auth ? 'Microsoft Account' : 'Offline Account'}
                                </p>
                                <p className="text-blue-400">
                                    {client.instance_count + ' instance' + (client.instance_count == 1 ? '' : 's')}
                                </p>
                            </div>
                        </div>
                    </div>
                ))}
            </div>
        </main>
    );
}

async function getClients(): Promise<ClientEntry[]> {
    return await invoke('get_clients')
}

export default function Clients() {
    const [showAuth, setShowAuth] = useState(false);
    const [clients, setClients] = useState<ClientEntry[]>([]);

    useEffect(() => {
        getClients()
            .then(e => setClients(e))
            .catch(e => console.error(e))
    }, []);

    return (
        <main className="p-4">
            <div className="flex justify-between items-center mb-4">
                <h1 className="text-2xl font-bold">Registered Clients</h1>
                <button className="bg-fuchsia-600 hover:bg-fuchsia-700 font-bold py-2 px-4 rounded
                                 transition duration-400 ease-in-out transform hover:scale-106 mx-4"
                        onClick={() => setShowAuth(true)}>
                    <i className="fa fa-plus"/>
                </button>
            </div>
            {clients && <ClientList clients={clients}/>}
            {showAuth && (
                <Auth onClose={() => setShowAuth(false)}
                      onFinish={(result) => {
                          let id = result[0];
                          let profile = result[1];
                          let prof = profile.asPreviewEntry(id);
                          setClients(
                              [
                                  ...clients,
                                  prof
                              ]
                          );
                          setShowAuth(false);
                      }
                }/>
            )}
        </main>
    );
}