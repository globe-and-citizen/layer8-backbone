<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
import { isLoggedIn, logout } from "@/utils.ts";
// import { checkPendingState } from "interceptor-wasm"
// import { ref } from 'vue'

// const providerUrls = ref(checkPendingState())
</script>

<template>
  <!-- Loader in top right corner -->
  <!-- <div v-if="providerUrls && providerUrls.length" class="loader-container" id="loaderContainer">
    <div v-for="url in providerUrls" :key="url" style="display: flex; align-items: center; gap: 8px;">
      <div class="loader"></div>
      <span class="loading-text">Connecting to: {{ url }}</span>
    </div>
  </div> -->

  <div style="position: relative; min-height: 4rem;">
    <nav>
      <RouterLink v-if="!isLoggedIn" to="/">Home</RouterLink>
      <RouterLink v-if="!isLoggedIn" to="/register">Register</RouterLink>
      <div v-if="isLoggedIn">
        <RouterLink to="/poems">Poems</RouterLink>
        <RouterLink to="/pictures">Pictures</RouterLink>
        <RouterLink to="/profile">Profile</RouterLink>
        <RouterLink to="/upload">Upload</RouterLink>
        <a href="/" @click.prevent="logout" style="padding:0 1rem;cursor:pointer;">Logout</a>
      </div>
    </nav>
  </div>

  <div style="height: 4rem;"></div>

  <div class="body">
    <RouterView />
  </div>

</template>

<style scoped>
nav {
  width: 100%;
  font-size: 1rem;
  text-align: right;
  padding: 1rem 0;
  margin-top: 0;
  position: static;
}

nav a.router-link-exact-active {
  color: var(--color-text);
}

nav a.router-link-exact-active:hover {
  background-color: transparent;
}

nav a {
  display: inline-block;
  padding: 0 1rem;
  border-left: 1px solid var(--color-border);
}

nav a:first-of-type {
  border: 0;
}

@media (min-width: 1024px) {
  nav {
    width: 100%;
    font-size: 1rem;
    text-align: right;
    padding: 1rem 0;
    margin-top: 0;
    position: static;
  }
}

/* Loader Container */
.loader-container {
  position: fixed;
  top: 20px;
  right: 20px;
  z-index: 9999;
  display: flex;
  align-items: center;
  gap: 10px;
  background: rgba(255, 255, 255, 0.9);
  padding: 15px 20px;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  backdrop-filter: blur(10px);
}


/* Spinning Circle Loader */
.loader {
  width: 24px;
  height: 24px;
  border: 3px solid #e3e3e3;
  border-top: 3px solid #007bff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% {
    transform: rotate(0deg);
  }

  100% {
    transform: rotate(360deg);
  }
}

/* Loading Text */
.loading-text {
  font-size: 14px;
  color: #333;
  font-weight: 500;
}

/* Hidden state */
.loader-container.hidden {
  display: none;
}

/* Alternative loader styles */
.loader.dots {
  border: none;
  width: 30px;
  height: 8px;
  background: linear-gradient(90deg, #007bff 0%, #007bff 33%, transparent 33%, transparent 66%, #007bff 66%);
  background-size: 12px 8px;
  animation: dots 1.2s infinite;
}

@keyframes dots {
  0% {
    background-position: 0 0;
  }

  50% {
    background-position: 12px 0;
  }

  100% {
    background-position: 24px 0;
  }
}

.loader.pulse {
  border: none;
  width: 24px;
  height: 24px;
  background: #007bff;
  border-radius: 50%;
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0% {
    transform: scale(0.8);
    opacity: 1;
  }

  50% {
    transform: scale(1.2);
    opacity: 0.5;
  }

  100% {
    transform: scale(0.8);
    opacity: 1;
  }
}
</style>
