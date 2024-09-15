import { DiscordSDK, CommandResponse } from '@discord/embedded-app-sdk';

type Auth = CommandResponse<'authenticate'>;

type User = {
    x: number,
    y: number,
}

type State = {
    users: { [k: string]: User }
}

const CLIENT_ID = process.env.CLIENT_ID!;
const discordSdk = new DiscordSDK(CLIENT_ID);

setupDiscordSdk().then((auth) => {
    console.log("AUTHED");
    appendVoiceChannelName();
    appendGuildAvatar(auth);
    loop(auth);
})

function load_avatar(auth: Auth): Promise<HTMLImageElement> {
    const avatar = new Image(80, 80);
    avatar.src = `https://cdn.discordapp.com/avatars/${auth.user.id}/${auth.user.avatar}.webp?size=80`;
    return new Promise((resolve) => {
        avatar.onload = () => {
            resolve(avatar);
        }
    });
}

async function loop(auth: Auth) {
    const app = document.querySelector<HTMLDivElement>('#app');
    if (!app) {
        throw new Error('Could not find #app element');
    }

    const WIDTH = 720;
    const HEIGHT = 480;

    const avatar = await load_avatar(auth);
    let state: State = await (await fetch("/.proxy/api/room/" + discordSdk.instanceId)).json();
    const protocol = window.location.protocol.startsWith("https") ? "wss" : "ws";
    const ws = new WebSocket(`${protocol}://${window.location.host}/.proxy/api/room/${discordSdk.instanceId}`);

    ws.onopen = (_event) => {
        state.users[auth.user.id] = {
            x: Math.floor(Math.random() * WIDTH),
            y: Math.floor(Math.random() * HEIGHT)
        }
        ws.send(JSON.stringify(state));
    }

    ws.onmessage = (event) => {
        try {
            state = JSON.parse(event.data);
        } catch (e) {
            console.warn("state parse error", e)
        }
    }

    const canvas = document.createElement("canvas");
    canvas.width = WIDTH;
    canvas.height = HEIGHT;
    app.appendChild(canvas);

    canvas.onclick = (event) => {
        const rect = canvas.getBoundingClientRect()
        const x = event.clientX - rect.left
        const y = event.clientY - rect.top;
        state.users[auth.user.id].x = x;
        state.users[auth.user.id].y = y;
        ws.send(JSON.stringify(state))
    }

    const ctx = canvas.getContext("2d")!;

    const inner = () => {
        ctx.clearRect(0, 0, WIDTH, HEIGHT);
        for (const user_id in state.users) {
            const user = state.users[user_id];
            ctx.drawImage(avatar, user.x, user.y);
        }

        requestAnimationFrame(inner)
    }
    requestAnimationFrame(inner);
}

async function setupDiscordSdk(): Promise<Auth> {
    await discordSdk.ready();

    // Authorize with Discord Client
    const { code } = await discordSdk.commands.authorize({
        client_id: CLIENT_ID,
        response_type: 'code',
        state: '',
        prompt: 'none',
        // More info on scopes here: https://discord.com/developers/docs/topics/oauth2#shared-resources-oauth2-scopes
        scope: [
            // Activities will launch through app commands and interactions of user-installable apps.
            // https://discord.com/developers/docs/tutorials/developing-a-user-installable-app#configuring-default-install-settings-adding-default-install-settings
            'applications.commands',

            // "applications.builds.upload",
            // "applications.builds.read",
            // "applications.store.update",
            // "applications.entitlements",
            // "bot",
            'identify',
            // "connections",
            // "email",
            // "gdm.join",
            'guilds',
            // "guilds.join",
            'guilds.members.read',
            // "messages.read",
            // "relationships.read",
            // 'rpc.activities.write',
            // "rpc.notifications.read",
            // "rpc.voice.write",
            'rpc.voice.read',
            // "webhook.incoming",
        ],
    });

    // Retrieve an access_token from your activity's server
    // /.proxy/ is prepended here in compliance with CSP
    // see https://discord.com/developers/docs/activities/development-guides#construct-a-full-url
    const response = await fetch('/.proxy/api/token', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            code,
        }),
    });
    const { access_token } = await response.json();

    // Authenticate with Discord client (using the access_token)
    const auth = await discordSdk.commands.authenticate({
        access_token,
    });

    if (auth == null) {
        throw new Error('Authenticate command failed');
    }

    return auth;
}

async function appendVoiceChannelName() {
    const app = document.querySelector<HTMLDivElement>('#app');
    if (!app) {
        throw new Error('Could not find #app element');
    }

    let activityChannelName = 'Unknown';

    // Requesting the channel in GDMs (when the guild ID is null) requires
    // the dm_channels.read scope which requires Discord approval.
    if (discordSdk.channelId != null && discordSdk.guildId != null) {
        // Over RPC collect info about the channel
        const channel = await discordSdk.commands.getChannel({
            channel_id: discordSdk.channelId,
        });
        if (channel.name != null) {
            activityChannelName = channel.name;
        }
    }

    // Update the UI with the name of the current voice channel
    const textTagString = `Activity Channel: "${activityChannelName}"`;
    const textTag = document.createElement('p');
    textTag.textContent = textTagString;
    app.appendChild(textTag);
}

/**
 * This function utilizes RPC and HTTP apis, in order show the current guild's avatar
 * Here are the steps:
 * 1. From RPC fetch the currently selected voice channel, which contains the voice channel's guild id
 * 2. From the HTTP API fetch a list of all of the user's guilds
 * 3. Find the current guild's info, including its "icon"
 * 4. Append to the UI an img tag with the related information
 */
async function appendGuildAvatar(auth: Auth) {
    const app = document.querySelector<HTMLDivElement>('#app');
    if (!app) {
        throw new Error('Could not find #app element');
    }

    // 1. From the HTTP API fetch a list of all of the user's guilds
    const guilds: Array<{ id: string; icon: string }> = await fetch(
        'https://discord.com/api/users/@me/guilds',
        {
            headers: {
                // NOTE: we're using the access_token provided by the "authenticate" command
                Authorization: `Bearer ${auth.access_token}`,
                'Content-Type': 'application/json',
            },
        },
    ).then((reply) => reply.json());

    // 2. Find the current guild's info, including it's "icon"
    const currentGuild = guilds.find((g) => g.id === discordSdk.guildId);

    // 3. Append to the UI an img tag with the related information
    if (currentGuild != null) {
        const guildImg = document.createElement('img');
        guildImg.setAttribute(
            'src',
            // More info on image formatting here: https://discord.com/developers/docs/reference#image-formatting
            `https://cdn.discordapp.com/icons/${currentGuild.id}/${currentGuild.icon}.webp?size=128`,
        );
        guildImg.setAttribute('width', '128px');
        guildImg.setAttribute('height', '128px');
        guildImg.setAttribute('style', 'border-radius: 50%;');
        app.appendChild(guildImg);
    }
}