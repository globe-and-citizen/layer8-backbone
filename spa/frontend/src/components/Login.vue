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
    </form>
  </div>
</template>

<script setup>
import {ref} from 'vue';
import {saveToken} from "@/utils.js";
import * as interceptorWasm from "interceptor-wasm"

const username = ref('');
const password = ref('');

const handleLogin = () => {
  // wasmBackend.login(username.value, password.value)
  //     .then(data => {
  //       let token = data.token || data["token"] || data.get("token");
  //       saveToken(token);
  //       alert(`Logged in as: ${username.value}`);
  //       location.href = '/profile';
  //     }).catch((err) => {
  //       alert(`An error occurred while logging in. ${err}`);
  // });

  // Request to localhost:6191/login, a simple fetch
  let body = {
    "username": username.value,
    "password": password.value
  }

  interceptorWasm.fetch('http://localhost:3000/login', {
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
