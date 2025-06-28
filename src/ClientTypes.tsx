export interface ClientEntry {
    id: string;
    username: string;
    auth: boolean;
    uuid?: string;
    instance_count: number;
}

export class MinecraftProfile {
    public uuid: string;
    public username: string;
    public skins: {};
    public capes: {};

    constructor(uuid: string, username: string, skins: {}, capes: {}) {
        this.uuid = uuid;
        this.username = username;
        this.skins = skins;
        this.capes = capes;
    }

    public asPreviewEntry(id: string): ClientEntry {
        return {
            id: id,
            username: this.username,
            auth: true,
            uuid: this.uuid,
            instance_count: 0,
        } as ClientEntry;
    }
}

export interface ServerEntry {
    name: string,
    ip: string,
    port: number
}