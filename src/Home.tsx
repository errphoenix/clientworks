import { useState } from "react";
import Clients from "./Clients"
import Servers from "./Servers"

export default function Home() {
    const [selectTab, setSelectTab] = useState("clients");
    let btn_1_col = selectTab === 'clients' ? 'bg-slate-900' : 'bg-slate-950';
    let btn_2_col = selectTab === 'servers' ? 'bg-slate-900' : 'bg-slate-950';

    return (
        <>
            <div className="flex justify-center items-center">
                <div className="mt-10 w-3/4 rounded-3xl bg-slate-800
                        drop-shadow-md drop-shadow-stone-950
                        min-h-10">
                    <div className="m-4">
                        <div className="flex drop-shadow-lg drop-shadow-stone-950">
                            <div className={btn_1_col+" mt-2 rounded-t-xl w-fit h-fit p-2 " +
                                "hover:text-zinc-400 select-none cursor-pointer " +
                                "ease-in-out duration-200"}
                                 onClick={() => setSelectTab("clients")}>
                                Clients
                            </div>
                            <div className={btn_2_col+" mt-2 rounded-t-xl w-fit h-fit p-2 " +
                                "hover:text-zinc-400 select-none cursor-pointer " +
                                "ease-in-out duration-200"}
                                 onClick={() => setSelectTab("servers")}>
                                Servers
                            </div>
                        </div>
                        <div className="bg-slate-900 min-h-32
                            rounded-b-xl rounded-tr-xl
                            drop-shadow-md drop-shadow-stone-950">
                            {selectTab === 'clients' ? <Clients/> : <Servers/>}
                        </div>
                    </div>
                </div>
            </div>
        </>
    )
}