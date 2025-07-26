import './assets/main.css'

import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { initEncryptedTunnel, ServiceProvider } from "interceptor-wasm"

let forward_proxy_url = import.meta.env.VITE_FORWARD_PROXY_URL || 'http://localhost:6191';
let backend_url = import.meta.env.VITE_BACKEND_URL || 'http://10.10.10.102:6193';

const layer8_ = async () => {
    try {
        let providers = [ServiceProvider.new(backend_url)];
        await initEncryptedTunnel(forward_proxy_url, providers).finally(() => {
            console.log('Encrypted tunnel initialized successfully');
        });
    } catch (err) {
        throw new Error(`Failed to initialize encrypted tunnel: ${err}`);
    }
};

// we need the promise to resolve before mounting the app
await layer8_();

const app = createApp(App)

app.config.globalProperties.$backend_url = backend_url;

app.use(router)

app.mount('#app')
