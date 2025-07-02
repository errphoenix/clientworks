# Clientworks

**A cross-platform, multi-client manager for Minecraft bots, designed for ease of use.**
**Built on Rust, using [azalea-rs](https://github.com/azalea-rs/azalea)**

## **Features**
* **Multiple Client Management** - Easily manage many Minecraft clients from a single interface.
* **Intuitive and Modern UI** - A user-friendly, modern interface built for simplicity and to ensure a smooth experience.
* **Low Resource Usage** - Ensure your system runs smoothly even with many clients online.
* **Chat History & Messaging** - View real-time chat when connected to servers, and send messages and commands.
* **Multi-Version Compatibility** - Supports various Minecraft versions.
* **Cross-Platform** - Clientworks is available on Windows, macOS and Linux.

## **How to Use (for Users)**
Getting started to use Clientworks is incredibly simple:
1. Simply go on the [Releases page](https://github.com/errphoenix/clientworks/releases) and download the suitable installer or executable for your system.
2.  Once installed, you're ready to go!

### Data directory
Clientworks stores its data in the system's data directory, this is:
* `%APPDATA%/herr.clientworks.client/` for **Windows**
* `~/.local/share/herr.clientworks.client/` for **Linux**
* `~/Library/Application Support/herr.clientworks.client/` for **macOS**

These directories store the clients list (`clients.json`), server list (`servers.json`) and authentication cache (`auth_cache.json`).

---

**(!) IMPORTANT** 
While the `clients.json` and `servers.json` don't contain any sensible information, the authentication cache, `auth_cache.json`, contains your account's access token, so do **not** share this file to people you don't trust, as it may be used to gain access to your account if the token is still valid.

---

## Upcoming Features
* **More in-depth instance page** | to view more details on the server you're connected to (such as player count, player list, uptime)
* **Better control over clients** | being able to control more aspects of the clients such as movement and interactions
* **Macros** | to execute complex actions with one click 
* **Swarms** | to quickly send groups of clients on a server (that's why the "Server" tab might seem pointless for now)
* **Scripting** | to script more complex bots and interactions with the server and other players

## Development Setup
If you wish to compile Clientworks from source:
1. **Clone the repository:**
```bash
git clone https://github.com/errphoenix/clientworks.git
cd clientworks
```
2. **Setup Tauri:** follow the official Tauri documentation [here](https://v1.tauri.app/v1/guides/getting-started/prerequisites/) for your system
3. **Install frontend dependencies:** simply run `npm install`
4. You should now be ready to run Clientworks in a development environment (using `npm run tauri dev` ) and build binaries for your platform (using `npm run tauri build`)
