<template>
    <div class="login-container">
        <h2>Login</h2>
        <form @submit.prevent="handleLogin">
            <div class="form-group">
                <label for="username">Username:</label>
                <input type="text" id="username" v-model="username" required/>
            </div>
            <div class="form-group">
                <label for="password">Password:</label>
                <input type="password" id="password" v-model="password" required/>
            </div>
            <button type="submit">Login</button>
            <button @click="loginWithLayer8Popup">Login With Layer8</button>
        </form>
    </div>
</template>

<script setup>
import {ref} from 'vue';
import {saveToken} from "@/utils.js";
import * as interceptorWasm from "layer8-interceptor-production";
import {getCurrentInstance} from 'vue';

const {appContext} = getCurrentInstance();
const backend_url = appContext.config.globalProperties.$backend_url;

const username = ref('');
const password = ref('');

const handleLogin = () => {
    let body = {
        "username": username.value,
        "password": password.value
    }

    interceptorWasm.fetch(`${backend_url}/login`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(body)
    }).then(response => {
        if (response.ok) {
            response.json().then(data => {
                let token = data.token || data["token"] || data.get("token");
                saveToken(token);
                alert(`Logged in as: ${username.value}`);
                location.href = '/profile';
            });
        } else {
            alert(`An error occurred while logging in. ${response.status}`);
        }
    });
};

const loginWithLayer8Popup = async () => {
    const response = await interceptorWasm.fetch(`${backend_url}/api/l8-login`)
    const data = await response.json()
    console.log(data)
    // create opener window
    const popup = window.open(data.authURL, "Login with Layer8", "width=1200,height=900");
    window.addEventListener("message", async (event) => {
        if (event.data.redirect_uri) {
            setTimeout(() => {
                console.log("Received message from popup:", event.data);
                interceptorWasm.fetch(`${backend_url}/l8-login-callback`, {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/Json",
                    },
                    body: JSON.stringify({
                        code: event.data.code
                    })
                })
                    .then(res => res.json())
                    .then(data => {
                        if (popup) {
                            popup.close();
                        }

                        let token = data.token || data["token"] || data.get("token");
                        saveToken(token);
                        alert(`Logged in as: ${data.profile.username}`);
                        location.href = '/profile';
                    })
                    .catch(err => console.log(err))
            }, 1000);
        }
    });
}
</script>

<style scoped>
.login-container {
    max-width: 400px;
    margin: 0 auto;
    padding: 20px;
    border: 1px solid #ccc;
    border-radius: 8px;
    background-color: #f9f9f9;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
}

h2 {
    text-align: center;
    margin-bottom: 20px;
    color: #333;
}

.form-group {
    margin-bottom: 15px;
}

label {
    display: block;
    margin-bottom: 5px;
    font-weight: bold;
    color: #555;
}

input {
    width: 100%;
    padding: 10px;
    border: 1px solid #ccc;
    border-radius: 4px;
    box-sizing: border-box;
}

button {
    width: 100%;
    padding: 10px;
    background-color: #007bff;
    color: #fff;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
}

button:hover {
    background-color: #0056b3;
}
</style>
