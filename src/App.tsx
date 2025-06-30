import "./App.css";
import { getCurrentWindow }                    from "@tauri-apps/api/window";
import {createBrowserRouter, RouterProvider,
    useNavigate}                               from "react-router";
import Home                                    from "./Home.tsx";
import ClientManager                           from "./ClientManager.tsx";

function Header() {
    const navigate = useNavigate();
    const window = getCurrentWindow();
    return (
        <main>

            <div id="titlebar" className="bg-slate-950 p-2 flex justify-between sticky top-0 z-50"
                 onMouseDown={(e) => {
                     if (e.buttons === 1) {
                         e.detail === 2 ? window.toggleMaximize() : window.startDragging();
                     }
                 }}>
                <div className="text-gray-500 text-sm p-2 select-none">
                    v0.2.0
                </div>
                <div
                    className="hover:bg-red-500 duration-300 p-2 mr-2 rounded cursor-pointer"
                    onClick={async () => await getCurrentWindow().close()}>
                    ✕
                </div>
            </div>
            <div className="flex justify-center items-center">
                <h1 className="font-mono text-6xl w-min mt-10
                      text-green-500 font-black italic
                      drop-shadow-[1px_3px_0px_#166534]
                      ease-out duration-600
                      hover:scale-125
                      hover:drop-shadow-[3px_3px_0px_#166534]
                      hover:contrast-180
                      cursor-default
                      select-none"
                    onClick={() => {
                        navigate('/');
                    }}
                >
                    Clientworks
                </h1>
            </div>
        </main>
    )
}

function App() {
    const router = createBrowserRouter([
        {
            path: "/",
            element:
                <>
                    <Header/>
                    <Home/>
                </>
        },
        {
            path: "/client/:id",
            element:
                <>
                    <Header/>
                    <ClientManager/>
                </>
        }
    ])

    return (
        <main>

            <RouterProvider router={router}/>
        </main>
    );
}

export default App;
