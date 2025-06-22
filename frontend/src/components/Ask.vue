<template>
  <div class="body">
    <h2>Ask me anything</h2>
    <div class="input-question">
      <div class="input-container">
        <input name="question" v-model="message" placeholder="Type your question here" @keyup.enter="ask()" />
        <span v-if="message" class="clear-button" @click="clear()">×</span>
      </div>

      <p>Answer:</p>
      <textarea name="answer" v-model="answer" readonly></textarea>
    </div>
  </div>
</template>

<script setup lang="ts">

import {ref} from 'vue'
import axios from "axios";

const message = ref("")
const answer = ref("")
const PROXY_ADDR = "https://localhost:6191/test-endpoint"

async function ask() {
  let res = await axios.post(PROXY_ADDR,
      {data: message.value},
      {
        headers: {
          'Content-Type': "application/json",
          'Accept': "application/json",
        }
      }
  )
  answer.value = res.data.data
}

function clear() {
  message.value = ""
  answer.value = ""
}

</script>

<style scoped>
div.body {
  margin: 5% 5%;
}

h2 {
  font-weight: 300;
  font-size: 2.6rem;
  position: relative;
  top: -50px;
}

.input-container {
  position: relative;
  width: 100%;
}

.input-container input {
  width: 100%;
  padding: 0.5rem 2.5rem 0.5rem 1rem;
  border: 2px solid #ccc;
  border-radius: 8px;
  font-size: 1rem;
  background-color: #1e1e1e;
  color: #f0f0f0;
  outline: none;
  transition: border-color 0.2s ease-in-out;
}

.input-container input:focus {
  border-color: #42b983;
}

.clear-button {
  position: absolute;
  right: 1rem;
  top: 50%;
  transform: translateY(-50%);
  color: #888;
  font-size: 1.2rem;
  cursor: pointer;
  user-select: none;
  background-color: transparent;
  border: none;
}

.clear-button:hover {
  color: #f0f0f0;
}

p {
  font-size: 1.25rem;
  margin: 5%;
}

textarea {
  width: 100%;
  min-height: 20rem; /* starting height */
  max-height: 30rem; /* optional: limit it */
  padding: 0.5rem 1rem;
  border-radius: 8px;
  font-size: 1rem;
  background-color: #1e1e1e;
  color: #f0f0f0;
  border: 2px solid #ccc;
  overflow-y: auto;
  resize: vertical; /* allow user to resize manually */
  box-sizing: border-box;
  line-height: 1.5;
  transition: border-color 0.2s ease-in-out;
  position: relative;
}


</style>
