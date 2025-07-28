import './assets/main.css'

import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { initEncryptedTunnel, ServiceProvider } from "interceptor-wasm"

let forward_proxy_url = import.meta.env.VITE_FORWARD_PROXY_URL || 'http://localhost:6191';
let backend_url = import.meta.env.VITE_BACKEND_URL || 'http://localhost:6193';

try {
    let providers = [ServiceProvider.new(backend_url)];
    initEncryptedTunnel(forward_proxy_url, providers);
} catch (err) {
    throw new Error(`Failed to initialize encrypted tunnel: ${err}`);
}

const app = createApp(App)
app.config.globalProperties.$backend_url = backend_url;
app.use(router)
app.mount('#app')
