import { invoke }              from "@tauri-apps/api/core";
import { listen, UnlistenFn }  from "@tauri-apps/api/event";
import { useState, useEffect } from "react";
import { MinecraftProfile}     from "./ClientTypes.tsx";

type Callback = {
    onClose: () => void;
    onFinish: (result: [string, MinecraftProfile]) => void;
}

type Credentials = {
    uri: string;
    code: string;
}

type AuthProgress = {
    state: string;
    message: string;
}

async function offlineAuth(username: string): Promise<[string, MinecraftProfile]> {
    return await invoke("auth_offline", { username: username })
}

async function finishAuth(key: string): Promise<[string, MinecraftProfile]> {
    return await invoke("auth_ms_finish", { loginKey: key, register: true })
}

async function cachedAuth(key: string): Promise<[string, MinecraftProfile]> {
    return await invoke("auth_ms_cache", { loginKey: key })
}

function Auth({onClose, onFinish}: Callback) {
    const [authType, setAuthType] = useState<"microsoft" | "offline">("microsoft");
    const [loginKey, setLoginKey] = useState("");
    const [isAuthenticating, setIsAuthenticating] = useState(false);
    const [credentials, setCredentials] = useState<{uri: string, code: string}>({uri: "", code: ""});
    const [label, setLabel] = useState({
        type: "",
        message: ""
    });

    const completeAuth = (result: [string, MinecraftProfile])=> {
        const [id, profileData] = result;
        const profile = new MinecraftProfile(
            profileData.uuid,
            profileData.username,
            profileData.skins || {},
            profileData.capes || {}
        );
        console.log(profile);
        setLabel({
            type: "success",
            message: `Authentication successful, ${profile.username}`
        });
        onFinish([id, profile]);
    }

    const handleAuthenticate = async () => {
        if (!loginKey.trim()) {
            setLabel({
                type: "error",
                message: "Please enter " + (authType === "microsoft" ? "an email" : "a username")
            });
            return;
        }
        setLabel({...label, message: ""});
        setIsAuthenticating(true);
        try {
            if (authType === "microsoft") {
                cachedAuth(loginKey)
                    .then(
                        result => completeAuth(result)
                    ).catch(e => {
                        console.log("Cached auth failed: " + e);
                        let credentialsRequest: Promise<Credentials> = invoke("auth_ms_init", {loginKey: loginKey});
                        credentialsRequest.then(e => {
                            setCredentials({
                                uri: e.uri,
                                code: e.code
                            });
                        })
                            .catch(e => {
                                throw (e)
                            });
                    });
            } else {
                await offlineAuth(loginKey)
                    .then(
                        result => completeAuth(result)
                    ).catch(e => { throw(e) });
            }
        } catch (e) {
            setLabel({
                type: "error",
                message: "Authentication failed: " + e
            });
        } finally {
            setIsAuthenticating(false);
        }
    };

    /**
     * This will sometimes throw the error: "Client <username> already exists",
     * due to authentication caching not being called properly.
     *
     * I have not noticed any bugs or negative effects, due to this, except for the
     * authentication dialog not automatically closing when authentication is complete.
     * It shouldn't cause any confusion to the user, though, as the green "Success" label
     * is still shown, they just need to close the dialog on their own.
     */
    const finalizeAuthentication = async () => {
        try {
            finishAuth(loginKey)
                .then(result => {
                    const [id, profileData] = result;
                    const profile = new MinecraftProfile(
                        profileData.uuid,
                        profileData.username,
                        profileData.skins || {},
                        profileData.capes || {}
                    );
                    console.log(profile);
                    setLabel({
                        type: "success",
                        message: `Authentication successful, ${profile.username}`
                    });
                    onFinish([id, profile]);
                }).catch(e => { throw(e) });
        } catch (e) {
            setLabel({
                type: "error",
                message: "Authentication failed: " + e
            });
        }
    };

    useEffect(() => {
        let unlisten: UnlistenFn;

        listen<AuthProgress>("auth-progress-update", (event) => {
            setLabel({
                type: event.payload.state,
                message: event.payload.message
            });
        }).then(unlistenFn => {
            unlisten = unlistenFn;
        });

        return () => {
            if (unlisten) {
                unlisten();
            }
        };
    }, []);

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
            <div className="fixed inset-0 bg-black bg-opacity-50" onClick={onClose}></div>
            <div className="relative w-full max-w-md p-8 space-y-6 bg-slate-800 rounded-xl shadow-lg z-10">
                <button
                    onClick={onClose}
                    className="absolute top-4 right-4 text-gray-400 hover:text-white"
                >
                    ✕
                </button>
                <h2 className="text-2xl font-bold text-center text-green-500">Add Minecraft Account</h2>
                <div className="flex justify-center space-x-4">
                    <label className="inline-flex items-center">
                        <input
                            type="radio"
                            value="microsoft"
                            checked={authType === "microsoft"}
                            onChange={(e) => setAuthType(e.target.value as "microsoft" | "offline")}
                            className="form-radio text-green-500 focus:ring-green-500"
                        />
                        <span className="ml-2 text-white">Microsoft Account</span>
                    </label>
                    <label className="inline-flex items-center">
                        <input
                            type="radio"
                            value="offline"
                            checked={authType === "offline"}
                            onChange={(e) => setAuthType(e.target.value as "microsoft" | "offline")}
                            className="form-radio text-green-500 focus:ring-green-500"
                        />
                        <span className="ml-2 text-white">Offline Account</span>
                    </label>
                </div>
                <div className="space-y-4">
                    {authType === "microsoft" ? (
                        <input
                            type="email"
                            value={loginKey}
                            onChange={(e) => setLoginKey(e.target.value)}
                            placeholder="Insert email"
                            title="This doesn't strictly have to be an email, it's used as a login key to cache your account token and avoid repeating the verification process. However, it's recommended to use your email to avoid confusion."
                            className="w-full px-4 py-2 rounded bg-slate-700 text-white
                            focus:outline-none focus:ring-2 focus:ring-green-500"
                        />
                    ) : (
                        <input
                            type="text"
                            value={loginKey}
                            maxLength={16}
                            onChange={(e) => setLoginKey(e.target.value)}
                            placeholder="Insert username"
                            className="w-full px-4 py-2 rounded bg-slate-700 text-white
                            focus:outline-none focus:ring-2 focus:ring-green-500"
                        />
                    )}
                    <button
                        onClick={handleAuthenticate}
                        disabled={isAuthenticating}
                        className="w-full px-4 py-2 text-white bg-green-600
                         rounded hover:bg-green-800 focus:outline-none
                         focus:ring-2 focus:ring-green-500 disabled:opacity-50
                         transition duration-300"
                    >
                        {isAuthenticating ? "Authenticating..." : "Authenticate"}
                    </button>
                    {credentials.uri && authType === "microsoft" && (
                        <div className="space-y-4">
                            <hr className="border-t border-gray-600 my-4"/>
                            <p className="text-white text-sm">
                                Follow the steps below to complete the authentication:
                            </p>
                            <ol className="list-decimal list-inside text-white text-sm space-y-2">
                                <li>Click the link below to open Microsoft authentication:</li>
                                <li>
                                    <a
                                        href={credentials.uri}
                                        className="text-blue-400 hover:text-blue-300 break-all"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                    >
                                        {credentials.uri}
                                    </a>
                                </li>
                                <li>Enter this code in the opened page:</li>
                                <code className="block ml-6 mr-6 bg-slate-700 p-2 rounded text-green-400 text-center">
                                    {credentials.code}
                                </code>
                                <li>Safely login into your Microsoft account.</li>
                            </ol>
                            <button
                                onClick={finalizeAuthentication}
                                className="w-full px-4 py-2 text-white bg-blue-600
                                         rounded hover:bg-blue-800 focus:outline-none
                                         focus:ring-2 focus:ring-blue-500
                                         duration-300"
                            >
                                Proceed
                            </button>
                        </div>
                    )}
                    {label &&
                        <p className={
                            `${label.type === "Error" ? "text-red-500" : "text-green-500"}
                             ${label.type === "Success" ? "font-bold" : ""} text-sm`}>
                            {label.type === "Success" ? label.type : label.message}
                        </p>
                    }
                </div>
            </div>
        </div>
    );
}

export default Auth;