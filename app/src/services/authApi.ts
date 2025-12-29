export interface DiscordAuthInitResponse {
    oauthUrl: string
    powPrefix: string
    powComplexity: number
    deliveryMethod: 'redirect' | 'polling'
}

export interface UserProfile {
    id: string
    username: string
    avatarUrl: string
}

export interface AuthorizationCallbackResponse {
    accessToken: string,
    id: string,
    username: string,
    avatarUrl: string,
    deliveryMethod: 'redirect' | 'polling',
    deliveryTarget: string,
}

function simulateDelay<T>(result: T, delayMs = 1200): Promise<T> {
    return new Promise((resolve) => setTimeout(() => resolve(result), delayMs))
}

export async function requestDiscordAuthInit(
    redirectUrl: string | null,
): Promise<DiscordAuthInitResponse> {
    if (!redirectUrl) {
        throw new Error('redirectUrl is required')
    }
    const resp = await fetch("/api/auth/prepare", {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify({
            redirectUrl,
            deliveryMethod: "redirect"
        }),
    });
    const data = await resp.json();

    if (!resp.ok) {
        const errorMessage = data.error || `${resp.status} ${resp.statusText}`;
        throw new Error(`Ошибка при подготовке авторизации: ${errorMessage}`);
    }

    const oauthUrl = data.oauthUrl;
    const powPrefix = data.powPrefix;
    const powComplexity = data.powComplexity;
    const deliveryMethod = data.deliveryMethod ?? "redirect";

    return simulateDelay({oauthUrl, powPrefix, powComplexity, deliveryMethod})
}

export async function fetchAuthorize(
    code: string,
    pow: { solution: string, prefix: string } | null,
): Promise<AuthorizationCallbackResponse> {
    if (!code) {
        throw new Error('Не найден параметр code в URL')
    }

    if (!pow) {
        throw new Error('Отсутствует nonce локальной сессии, авторизация устарела')
    }

    const resp = await fetch("/api/auth/authorize", {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify({ discordCode: code, powSolution: pow.solution, powPrefix: pow.prefix }),
    });

    const data = await resp.json();

    if (!resp.ok) {
        const errorMessage = data.error || `${resp.status} ${resp.statusText}`;
        throw new Error(`Ошибка при авторизации: ${errorMessage}`);
    }

    return simulateDelay(data as AuthorizationCallbackResponse)
}


export async function startPowComputation(user: UserProfile): Promise<void> {
    const failChance = 0.15

    await new Promise((resolve) => setTimeout(resolve, 2000))

    if (Math.random() < failChance) {
        throw new Error(`Сбой при выполнении Proof-of-Work для пользователя ${user.username}`)
    }
}
