import {computed, getCurrentInstance} from "vue";
import * as interceptorWasm from "layer8-interceptor-production";

export async function interceptorFetch(
    url: string,
    options: RequestInit = {}
): Promise<Response> {
    options.credentials = "include"; // VERY IMPORTANT for cookies to be sent
    if (import.meta.env.VITE_ENABLE_LAYER8 === 'true') {
        return (await interceptorWasm.fetch(url, options)) as Response;
    } else {
        return (await fetch(url, options)) as Response;
    }
}

function getCookie(name: string): string | undefined {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; ${name}=`);

    if (parts.length === 2)
        return parts.pop()?.split(';').shift();
}

export function saveToken(token: string) {
    document.cookie = `jwt=${token}; path=/;`;
}

export function getToken(name: string): string | undefined {
    let cookie = getCookie(name);
    if (cookie) {
        return `Bearer ${cookie}`
    }
    return undefined;
}

export const isLoggedIn = computed(() => {
    // return getCookie("jwt") !== undefined && getCookie("jwt")?.length > 0;
    return localStorage.getItem('username') !== null;
})

export function logout() {
    // document.cookie = 'jwt=; expires=Thu, 01 Jan 1970 00:00:01 GMT;';
    // alert('Logged out successfully.')
    // location.href = '/'
    interceptorFetch(`${import.meta.env.VITE_BACKEND_URL}/logout`, {method: 'POST'})
        .then(res => res.json())
        .then(data => {
            console.log(data);
            localStorage.removeItem('username');
            alert('Logged out successfully.')
            document.cookie = "demo.spa=; Path=/; Max-Age=0"
            location.href = '/'
        }).catch(err => console.error(err));
}

export function setUser(username: string | null) {
    if (username) {
        localStorage.setItem('username', username);
    } else {
        localStorage.removeItem('username');
    }
}

export function getAuthUsername(): string {
    // const token = getToken('jwt');
    // if (!token) {
    //     console.error('No token found');
    //     return;
    // }
    //
    // const payload = JSON.parse(atob(token.split('.')[1]));
    // const username = payload.username;
    return localStorage.getItem('username') || '';
}

export async function checkAuth() {
    // return getCookie("jwt") !== undefined && getCookie("jwt")?.length > 0; // todo
    let res = await interceptorFetch(`${import.meta.env.VITE_BACKEND_URL}/me`)
    if (res.status < 400) {
        const data = await res.json();
        setUser(data.user?.username);
        return true;
    }
    setUser(null);
    return false;

}

export function toBlob(filename: string, bytes: Uint8Array | number[] | ArrayBuffer): Blob | null {
    if (bytes && bytes.length > 0) {
        if (!(bytes instanceof Uint8Array)) {
            bytes = new Uint8Array(bytes);
        }
        return new File([bytes], filename, {type: "image/jpeg"});
    }
    return null
}

export function toImageUrl(filename: string, bytes: Uint8Array | number[] | ArrayBuffer): string | null {
    let blob = toBlob(filename, bytes);
    if (blob == null) {
        return null
    }
    return URL.createObjectURL(blob)
}

export function revokeURL(url: string) {
    URL.revokeObjectURL(url);
}


// let backendConfig = new interceptorWasm.WGPBackendConfig();
// backendConfig.base_url = "http://localhost:6191";
// backendConfig.login = "/login";
// backendConfig.register = "/register";
// backendConfig.get_image_path = "/images?id={}";
// backendConfig.get_images_path = "/images";
// backendConfig.get_poem_path = "/poems?id={}";
// backendConfig.get_poems_path = "/poems";
// backendConfig.get_profile_path = "/profile";

// export const wasmBackend = new interceptorWasm.WGPBackend(backendConfig);


