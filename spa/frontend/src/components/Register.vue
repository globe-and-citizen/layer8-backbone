<template>
  <div class="register-container">
    <h2>Register</h2>
    <form @submit.prevent="handleRegister">
      <div class="form-group">
        <label for="username">Username:</label>
        <input type="text" id="username" v-model="username" required/>
      </div>
      <div class="form-group">
        <label for="password">Password:</label>
        <input type="password" id="password" v-model="password" required/>
      </div>
      <button type="submit">Register</button>
    </form>
  </div>
</template>

<script setup>
import {ref} from 'vue';

const username = ref('');
const password = ref('');

const handleRegister = () => {
  // wasmBackend.register(username.value, password.value)
  //     .then(response => {
  //       alert("Registration successful! You can now log in.");
  //       location.href = '/';
  //     }).catch(() => {
  //   alert('An error occurred while registering.');
  // });

  // Request to localhost:6191/register, a simple fetch
  let body = {
    "username": username.value,
    "password": password.value
  }

  fetch('https://localhost:6191/register', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(body)
  }).then(response => {
    if (response.ok) {
      alert(`Registered as: ${username.value}`);
      location.href = '/';
    } else {
      alert(`An error occurred while registering. ${response.status}`);
    }
  });
};
</script>

<style scoped>
.register-container {
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
  background-color: #28a745;
  color: #fff;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 16px;
}

button:hover {
  background-color: #218838;
}
</style>
